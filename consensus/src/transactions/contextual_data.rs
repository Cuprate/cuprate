//! # Contextual Data
//!
//! This module fills [`TxRingMembersInfo`] which is a struct made up from blockchain information about the
//! ring members of inputs. This module does minimal consensus checks, only when needed, and should not be relied
//! upon to do any.
//!
//! The data collected by this module can be used to perform consensus checks.
//!
//! ## Why not use the context service?
//!
//! Because this data is unique for *every* transaction and the context service is just for blockchain state data.
//!
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use monero_serai::transaction::{Input, Timelock};
use tower::ServiceExt;
use tracing::instrument;

use cuprate_consensus_rules::transactions::Rings;
use cuprate_consensus_rules::{
    transactions::{
        get_absolute_offsets, insert_ring_member_ids, DecoyInfo, TransactionError,
        TxRingMembersInfo,
    },
    ConsensusError, HardFork, TxVersion,
};
use cuprate_types::{
    service::{BCReadRequest, BCResponse},
    OutputOnChain,
};

use crate::{transactions::TransactionVerificationData, Database, ExtendedConsensusError};

/// Get the ring members for the inputs from the outputs on the chain.
///
/// Will error if `outputs` does not contain the outputs needed.
fn get_ring_members_for_inputs(
    get_outputs: impl Fn(u64, u64) -> Option<OutputOnChain>,
    inputs: &[Input],
) -> Result<Vec<Vec<OutputOnChain>>, TransactionError> {
    inputs
        .iter()
        .map(|inp| match inp {
            Input::ToKey {
                amount,
                key_offsets,
                ..
            } => {
                let offsets = get_absolute_offsets(key_offsets)?;
                Ok(offsets
                    .iter()
                    .map(|offset| {
                        get_outputs(amount.unwrap_or(0), *offset)
                            .ok_or(TransactionError::RingMemberNotFoundOrInvalid)
                    })
                    .collect::<Result<_, TransactionError>>()?)
            }
            _ => Err(TransactionError::IncorrectInputType),
        })
        .collect::<Result<_, TransactionError>>()
}

/// Construct a [`TxRingMembersInfo`] struct.
///
/// The used outs must be all the ring members used in the transactions inputs.
pub fn new_ring_member_info(
    used_outs: Vec<Vec<OutputOnChain>>,
    decoy_info: Option<DecoyInfo>,
    tx_version: TxVersion,
) -> Result<TxRingMembersInfo, TransactionError> {
    Ok(TxRingMembersInfo {
        youngest_used_out_height: used_outs
            .iter()
            .map(|inp_outs| {
                inp_outs
                    .iter()
                    // the output with the highest height is the youngest
                    .map(|out| out.height)
                    .max()
                    .expect("Input must have ring members")
            })
            .max()
            .expect("Tx must have inputs"),
        time_locked_outs: used_outs
            .iter()
            .flat_map(|inp_outs| {
                inp_outs
                    .iter()
                    .filter_map(|out| match out.time_lock {
                        Timelock::None => None,
                        lock => Some(lock),
                    })
                    .collect::<Vec<_>>()
            })
            .collect(),
        rings: new_rings(used_outs, tx_version)?,
        decoy_info,
    })
}

/// Builds the [`Rings`] for the transaction inputs, from the given outputs.
fn new_rings(
    outputs: Vec<Vec<OutputOnChain>>,
    tx_version: TxVersion,
) -> Result<Rings, TransactionError> {
    Ok(match tx_version {
        TxVersion::RingSignatures => Rings::Legacy(
            outputs
                .into_iter()
                .map(|inp_outs| {
                    inp_outs
                        .into_iter()
                        .map(|out| out.key.ok_or(TransactionError::RingMemberNotFoundOrInvalid))
                        .collect::<Result<Vec<_>, TransactionError>>()
                })
                .collect::<Result<Vec<_>, TransactionError>>()?,
        ),
        TxVersion::RingCT => Rings::RingCT(
            outputs
                .into_iter()
                .map(|inp_outs| {
                    inp_outs
                        .into_iter()
                        .map(|out| {
                            Ok([
                                out.key
                                    .ok_or(TransactionError::RingMemberNotFoundOrInvalid)?,
                                out.commitment,
                            ])
                        })
                        .collect::<Result<_, TransactionError>>()
                })
                .collect::<Result<_, _>>()?,
        ),
    })
}

/// Retrieves the [`TxRingMembersInfo`] for the inputted [`TransactionVerificationData`].
///
/// This function batch gets all the ring members for the inputted transactions and fills in data about
/// them.
pub async fn batch_get_ring_member_info<D: Database>(
    txs_verification_data: impl Iterator<Item = &Arc<TransactionVerificationData>> + Clone,
    hf: &HardFork,
    mut database: D,
) -> Result<Vec<TxRingMembersInfo>, ExtendedConsensusError> {
    let mut output_ids = HashMap::new();

    for tx_v_data in txs_verification_data.clone() {
        insert_ring_member_ids(&tx_v_data.tx.prefix.inputs, &mut output_ids)
            .map_err(ConsensusError::Transaction)?;
    }

    let BCResponse::Outputs(outputs) = database
        .ready()
        .await?
        .call(BCReadRequest::Outputs(output_ids))
        .await?
    else {
        panic!("Database sent incorrect response!")
    };

    let BCResponse::NumberOutputsWithAmount(outputs_with_amount) = database
        .ready()
        .await?
        .call(BCReadRequest::NumberOutputsWithAmount(
            outputs.keys().copied().collect(),
        ))
        .await?
    else {
        panic!("Database sent incorrect response!")
    };

    Ok(txs_verification_data
        .map(move |tx_v_data| {
            let numb_outputs = |amt| outputs_with_amount.get(&amt).copied().unwrap_or(0);

            let ring_members_for_tx = get_ring_members_for_inputs(
                |amt, idx| outputs.get(&amt)?.get(&idx).copied(),
                &tx_v_data.tx.prefix.inputs,
            )
            .map_err(ConsensusError::Transaction)?;

            let decoy_info = if hf != &HardFork::V1 {
                // this data is only needed after hard-fork 1.
                Some(
                    DecoyInfo::new(&tx_v_data.tx.prefix.inputs, numb_outputs, hf)
                        .map_err(ConsensusError::Transaction)?,
                )
            } else {
                None
            };

            new_ring_member_info(ring_members_for_tx, decoy_info, tx_v_data.version)
                .map_err(ConsensusError::Transaction)
        })
        .collect::<Result<_, _>>()?)
}

/// Refreshes the transactions [`TxRingMembersInfo`], if needed.
///
/// # Panics
/// This functions panics if `hf == HardFork::V1` as decoy info
/// should not be needed for V1.
#[instrument(level = "debug", skip_all)]
pub async fn batch_get_decoy_info<'a, D: Database + Clone + Send + 'static>(
    txs_verification_data: &'a [Arc<TransactionVerificationData>],
    hf: HardFork,
    mut database: D,
) -> Result<impl Iterator<Item = Result<DecoyInfo, ConsensusError>> + 'a, ExtendedConsensusError> {
    // decoy info is not needed for V1.
    assert_ne!(hf, HardFork::V1);

    tracing::debug!(
        "Retrieving decoy info for {} txs.",
        txs_verification_data.len()
    );

    // Get all the different input amounts.
    let unique_input_amounts = txs_verification_data
        .iter()
        .flat_map(|tx_info| {
            tx_info.tx.prefix.inputs.iter().map(|input| match input {
                Input::ToKey { amount, .. } => amount.unwrap_or(0),
                _ => 0,
            })
        })
        .collect::<HashSet<_>>();

    tracing::debug!(
        "Getting the amount of outputs with certain amounts for {} amounts",
        unique_input_amounts.len()
    );

    let BCResponse::NumberOutputsWithAmount(outputs_with_amount) = database
        .ready()
        .await?
        .call(BCReadRequest::NumberOutputsWithAmount(
            unique_input_amounts.into_iter().collect(),
        ))
        .await?
    else {
        panic!("Database sent incorrect response!")
    };

    Ok(txs_verification_data.iter().map(move |tx_v_data| {
        DecoyInfo::new(
            &tx_v_data.tx.prefix.inputs,
            |amt| outputs_with_amount.get(&amt).copied().unwrap_or(0),
            &hf,
        )
        .map_err(ConsensusError::Transaction)
    }))
}

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

use std::{borrow::Cow, collections::HashSet};

use indexmap::IndexMap;
use monero_serai::transaction::{Input, Timelock};
use tower::ServiceExt;
use tracing::instrument;

use cuprate_consensus_rules::{
    transactions::{
        get_absolute_offsets, insert_ring_member_ids, DecoyInfo, Rings, TransactionError,
        TxRingMembersInfo,
    },
    ConsensusError, HardFork, TxVersion,
};

use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    output_cache::OutputCache,
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
            Input::Gen(_) => Err(TransactionError::IncorrectInputType),
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
                        Timelock::Block(_) | Timelock::Time(_) => Some(out.time_lock),
                    })
                    .collect::<Vec<_>>()
            })
            .collect(),
        rings: new_rings(used_outs, tx_version),
        decoy_info,
    })
}

/// Builds the [`Rings`] for the transaction inputs, from the given outputs.
fn new_rings(outputs: Vec<Vec<OutputOnChain>>, tx_version: TxVersion) -> Rings {
    match tx_version {
        TxVersion::RingSignatures => Rings::Legacy(
            outputs
                .into_iter()
                .map(|inp_outs| inp_outs.into_iter().map(|out| out.key).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
        ),
        TxVersion::RingCT => Rings::RingCT(
            outputs
                .into_iter()
                .map(|inp_outs| {
                    inp_outs
                        .into_iter()
                        .map(|out| [out.key, out.commitment])
                        .collect::<_>()
                })
                .collect::<_>(),
        ),
    }
}

/// Retrieves an [`OutputCache`] for the list of transactions.
///
/// The [`OutputCache`] will only contain the outputs currently in the blockchain.
pub async fn get_output_cache<D: Database>(
    txs_verification_data: impl Iterator<Item = &TransactionVerificationData>,
    mut database: D,
) -> Result<OutputCache, ExtendedConsensusError> {
    let mut outputs = IndexMap::new();

    for tx_v_data in txs_verification_data {
        insert_ring_member_ids(&tx_v_data.tx.prefix().inputs, &mut outputs)
            .map_err(ConsensusError::Transaction)?;
    }

    let BlockchainResponse::Outputs(outputs) = database
        .ready()
        .await?
        .call(BlockchainReadRequest::Outputs {
            outputs,
            get_txid: false,
        })
        .await?
    else {
        unreachable!();
    };

    Ok(outputs)
}

/// Retrieves the [`TxRingMembersInfo`] for the inputted [`TransactionVerificationData`].
///
/// This function batch gets all the ring members for the inputted transactions and fills in data about
/// them.
pub async fn batch_get_ring_member_info<D: Database>(
    txs_verification_data: impl Iterator<Item = &TransactionVerificationData> + Clone,
    hf: HardFork,
    mut database: D,
    cache: Option<&OutputCache>,
) -> Result<Vec<TxRingMembersInfo>, ExtendedConsensusError> {
    let mut outputs = IndexMap::new();

    for tx_v_data in txs_verification_data.clone() {
        insert_ring_member_ids(&tx_v_data.tx.prefix().inputs, &mut outputs)
            .map_err(ConsensusError::Transaction)?;
    }

    let outputs = if let Some(cache) = cache {
        Cow::Borrowed(cache)
    } else {
        let BlockchainResponse::Outputs(outputs) = database
            .ready()
            .await?
            .call(BlockchainReadRequest::Outputs {
                outputs,
                get_txid: false,
            })
            .await?
        else {
            unreachable!();
        };

        Cow::Owned(outputs)
    };

    Ok(txs_verification_data
        .map(move |tx_v_data| {
            let numb_outputs = |amt| outputs.number_outs_with_amount(amt);

            let ring_members_for_tx = get_ring_members_for_inputs(
                |amt, idx| outputs.get_output(amt, idx).copied(),
                &tx_v_data.tx.prefix().inputs,
            )
            .map_err(ConsensusError::Transaction)?;

            let decoy_info = if hf == HardFork::V1 {
                None
            } else {
                // this data is only needed after hard-fork 1.
                Some(
                    DecoyInfo::new(&tx_v_data.tx.prefix().inputs, numb_outputs, hf)
                        .map_err(ConsensusError::Transaction)?,
                )
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
pub async fn batch_get_decoy_info<'a, 'b, D: Database>(
    txs_verification_data: impl Iterator<Item = &'a TransactionVerificationData> + Clone,
    hf: HardFork,
    mut database: D,
    cache: Option<&'b OutputCache>,
) -> Result<
    impl Iterator<Item = Result<DecoyInfo, ConsensusError>> + sealed::Captures<(&'a (), &'b ())>,
    ExtendedConsensusError,
> {
    // decoy info is not needed for V1.
    assert_ne!(hf, HardFork::V1);

    // Get all the different input amounts.
    let unique_input_amounts = txs_verification_data
        .clone()
        .flat_map(|tx_info| {
            tx_info.tx.prefix().inputs.iter().map(|input| match input {
                Input::ToKey { amount, .. } => amount.unwrap_or(0),
                Input::Gen(_) => 0,
            })
        })
        .collect::<HashSet<_>>();

    tracing::debug!(
        "Getting the amount of outputs with certain amounts for {} amounts",
        unique_input_amounts.len()
    );

    let outputs_with_amount = if let Some(cache) = cache {
        unique_input_amounts
            .into_iter()
            .map(|amount| (amount, cache.number_outs_with_amount(amount)))
            .collect()
    } else {
        let BlockchainResponse::NumberOutputsWithAmount(outputs_with_amount) = database
            .ready()
            .await?
            .call(BlockchainReadRequest::NumberOutputsWithAmount(
                unique_input_amounts.into_iter().collect(),
            ))
            .await?
        else {
            unreachable!();
        };

        outputs_with_amount
    };

    Ok(txs_verification_data.map(move |tx_v_data| {
        DecoyInfo::new(
            &tx_v_data.tx.prefix().inputs,
            |amt| outputs_with_amount.get(&amt).copied().unwrap_or(0),
            hf,
        )
        .map_err(ConsensusError::Transaction)
    }))
}

mod sealed {
    /// TODO: Remove me when 2024 Rust
    ///
    /// <https://rust-lang.github.io/rfcs/3498-lifetime-capture-rules-2024.html#the-captures-trick>
    pub trait Captures<U> {}
    impl<T: ?Sized, U> Captures<U> for T {}
}

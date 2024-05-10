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

use monero_serai::transaction::Input;
use tower::ServiceExt;
use tracing::instrument;

use cuprate_consensus_rules::{
    transactions::{
        get_ring_members_for_inputs, insert_ring_member_ids, DecoyInfo, TxRingMembersInfo,
    },
    ConsensusError, HardFork,
};

use crate::{
    transactions::TransactionVerificationData, Database, DatabaseRequest, DatabaseResponse,
    ExtendedConsensusError,
};

/// Retrieves the [`TxRingMembersInfo`] for the inputted [`TransactionVerificationData`].
///
/// This function batch gets all the ring members for the inputted transactions and fills in data about
/// them.
pub async fn batch_get_ring_member_info<'a, 'b, D: Database>(
    txs_verification_data: impl Iterator<Item = &'a Arc<TransactionVerificationData>> + Clone,
    hf: &'b HardFork,
    mut database: D,
) -> Result<Vec<TxRingMembersInfo>, ExtendedConsensusError> {
    let mut output_ids = HashMap::new();

    for tx_v_data in txs_verification_data.clone() {
        insert_ring_member_ids(&tx_v_data.tx.prefix.inputs, &mut output_ids)
            .map_err(ConsensusError::Transaction)?;
    }

    let DatabaseResponse::Outputs(outputs) = database
        .ready()
        .await?
        .call(DatabaseRequest::Outputs(output_ids))
        .await?
    else {
        panic!("Database sent incorrect response!")
    };

    let DatabaseResponse::NumberOutputsWithAmount(outputs_with_amount) = database
        .ready()
        .await?
        .call(DatabaseRequest::NumberOutputsWithAmount(
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

            TxRingMembersInfo::new(ring_members_for_tx, decoy_info, tx_v_data.version)
                .map_err(ConsensusError::Transaction)
        })
        .collect::<Result<_, _>>()?)
}

/// Refreshes the transactions [`TxRingMembersInfo`], if needed.
#[instrument(level = "debug", skip_all)]
pub async fn batch_get_decoy_info<'a, 'b, D: Database + Clone + Send + 'static>(
    txs_verification_data: &'a [Arc<TransactionVerificationData>],
    hf: &'b HardFork,
    mut database: D,
) -> Result<
    impl Iterator<Item = Result<DecoyInfo, ConsensusError>> + Captures<(&'a (), &'b ())>,
    ExtendedConsensusError,
> {
    // decoy info is not needed for V1.
    assert_ne!(hf, &HardFork::V1);

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

    let DatabaseResponse::NumberOutputsWithAmount(outputs_with_amount) = database
        .ready()
        .await?
        .call(DatabaseRequest::NumberOutputsWithAmount(
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
            hf,
        )
        .map_err(ConsensusError::Transaction)
    }))
}

/// TODO: Remove Me .
/// <https://rust-lang.github.io/rfcs/3498-lifetime-capture-rules-2024.html#the-captures-trick>
pub(crate) trait Captures<U> {}
impl<T: ?Sized, U> Captures<U> for T {}

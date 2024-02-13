//! # Contextual Data
//!
//! This module contains [`TxRingMembersInfo`] which is a struct made up from blockchain information about the
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
    ops::Deref,
    sync::Arc,
};

use monero_serai::transaction::Input;
use tower::ServiceExt;

use monero_consensus::{
    transactions::{
        get_ring_members_for_inputs, insert_ring_member_ids, DecoyInfo, TxRingMembersInfo,
    },
    ConsensusError, HardFork,
};

use crate::{
    context::ReOrgToken,
    transactions::{output_cache::OutputCache, TransactionVerificationData},
    Database, DatabaseRequest, DatabaseResponse, ExtendedConsensusError,
};

pub async fn batch_refresh_ring_member_info<'a, D: Database + Clone + Send + Sync + 'static>(
    txs_verification_data: &'a [Arc<TransactionVerificationData>],
    hf: &HardFork,
    re_org_token: ReOrgToken,
    mut database: D,
    out_cache: Option<&OutputCache<'a>>,
) -> Result<(), ExtendedConsensusError> {
    let (txs_needing_full_refresh, txs_needing_partial_refresh) =
        ring_member_info_needing_refresh(txs_verification_data, hf);

    if !txs_needing_full_refresh.is_empty() {
        batch_fill_ring_member_info(
            txs_needing_full_refresh.iter(),
            hf,
            re_org_token,
            database.clone(),
            out_cache,
        )
        .await?;
    }

    let unique_input_amounts = txs_needing_partial_refresh
        .iter()
        .flat_map(|tx_info| {
            tx_info
                .tx
                .prefix
                .inputs
                .iter()
                .map(|input| match input {
                    Input::ToKey { amount, .. } => amount.unwrap_or(0),
                    _ => 0,
                })
                .collect::<HashSet<_>>()
        })
        .collect::<HashSet<_>>();

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

    for tx_v_data in txs_needing_partial_refresh {
        let decoy_info = if hf != &HardFork::V1 {
            // this data is only needed after hard-fork 1.
            Some(
                DecoyInfo::new(&tx_v_data.tx.prefix.inputs, &outputs_with_amount, hf)
                    .map_err(ConsensusError::Transaction)?,
            )
        } else {
            None
        };

        // Temporarily acquirer the mutex lock to add the ring member info.
        tx_v_data
            .rings_member_info
            .lock()
            .unwrap()
            .as_mut()
            // this unwrap is safe as otherwise this would require a full refresh not a partial one.
            .unwrap()
            .0
            .decoy_info = decoy_info;
    }

    Ok(())
}

/// This function returns the transaction verification datas that need refreshing.
///
/// The first returned vec needs a full refresh.
/// The second returned vec only needs a partial refresh.
///
/// A full refresh is a refresh of all the ring members and the decoy info.
/// A partial refresh is just a refresh of the decoy info.
fn ring_member_info_needing_refresh(
    txs_verification_data: &[Arc<TransactionVerificationData>],
    hf: &HardFork,
) -> (
    Vec<Arc<TransactionVerificationData>>,
    Vec<Arc<TransactionVerificationData>>,
) {
    let mut txs_needing_full_refresh = Vec::new();
    let mut txs_needing_partial_refresh = Vec::new();

    for tx in txs_verification_data {
        let tx_ring_member_info = tx.rings_member_info.lock().unwrap();

        // if we don't have ring members or if a re-org has happened do a full refresh.
        if let Some(tx_ring_member_info) = tx_ring_member_info.deref() {
            if tx_ring_member_info.1.reorg_happened() {
                txs_needing_full_refresh.push(tx.clone());
                continue;
            }
        } else {
            txs_needing_full_refresh.push(tx.clone());
            continue;
        }

        // if any input does not have a 0 amount do a partial refresh, this is because some decoy info
        // data is based on the amount of non-ringCT outputs at a certain point.
        // Or if a hf has happened as this will change the default minimum decoys.
        if &tx_ring_member_info
            .as_ref()
            .expect("We just checked if this was None")
            .0
            .hf
            != hf
            || tx.tx.prefix.inputs.iter().any(|inp| match inp {
                Input::Gen(_) => false,
                Input::ToKey { amount, .. } => amount.is_some(),
            })
        {
            txs_needing_partial_refresh.push(tx.clone());
        }
    }

    (txs_needing_full_refresh, txs_needing_partial_refresh)
}

/// Fills the `rings_member_info` field on the inputted [`TransactionVerificationData`].
///
/// This function batch gets all the ring members for the inputted transactions and fills in data about
/// them.
pub async fn batch_fill_ring_member_info<'a, D: Database + Clone + Send + Sync + 'static>(
    txs_verification_data: impl Iterator<Item = &Arc<TransactionVerificationData>> + Clone,
    hf: &HardFork,
    re_org_token: ReOrgToken,
    mut database: D,
    out_cache: Option<&OutputCache<'a>>,
) -> Result<(), ExtendedConsensusError> {
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

    for tx_v_data in txs_verification_data {
        let ring_members_for_tx = get_ring_members_for_inputs(
            |amt, idx| {
                if let Some(cached_outs) = out_cache {
                    if let Some(out) = cached_outs.get_out(amt, idx) {
                        return Some(out);
                    }
                }

                outputs.get(&amt)?.get(&idx)
            },
            &tx_v_data.tx.prefix.inputs,
        )
        .map_err(ConsensusError::Transaction)?;

        let decoy_info = if hf != &HardFork::V1 {
            // this data is only needed after hard-fork 1.
            Some(
                DecoyInfo::new(&tx_v_data.tx.prefix.inputs, &outputs_with_amount, hf)
                    .map_err(ConsensusError::Transaction)?,
            )
        } else {
            None
        };

        // Temporarily acquirer the mutex lock to add the ring member info.
        let _ = tx_v_data.rings_member_info.lock().unwrap().insert((
            TxRingMembersInfo::new(ring_members_for_tx, decoy_info, tx_v_data.version, *hf)
                .map_err(ConsensusError::Transaction)?,
            re_org_token.clone(),
        ));
    }

    Ok(())
}

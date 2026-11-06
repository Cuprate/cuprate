//! Read-query helpers for the transaction pool.
//! A shared module so that `block_template.rs` can access these helper methods.
use std::collections::HashMap;

use anyhow::Error;
use cuprate_txpool::{
    service::{
        interface::{TxpoolReadRequest, TxpoolReadResponse},
        TxpoolReadHandle,
    },
    TxEntry,
};
use cuprate_types::TransactionVerificationData;
use tower::{Service, ServiceExt};

/// [`TxpoolReadRequest::Backlog`]
pub(crate) async fn backlog(txpool_read: &mut TxpoolReadHandle) -> Result<Vec<TxEntry>, Error> {
    let TxpoolReadResponse::Backlog(tx_entries) = txpool_read
        .ready()
        .await?
        .call(TxpoolReadRequest::Backlog)
        .await?
    else {
        unreachable!();
    };

    Ok(tx_entries)
}

/// [`TxpoolReadRequest::TxsForBlock`].
///
/// Returns verification data keyed by transaction hash, plus the indexes in `tx_hashes` that
/// were not present in the txpool.
pub(crate) async fn txs_for_block(
    txpool_read: &mut TxpoolReadHandle,
    tx_hashes: Vec<[u8; 32]>,
) -> Result<(HashMap<[u8; 32], TransactionVerificationData>, Vec<usize>), Error> {
    let TxpoolReadResponse::TxsForBlock { txs, missing } = txpool_read
        .ready()
        .await?
        .call(TxpoolReadRequest::TxsForBlock(tx_hashes))
        .await?
    else {
        unreachable!();
    };

    Ok((txs, missing))
}

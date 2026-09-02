//! Functions to send [`TxpoolReadRequest`]s.
use anyhow::{anyhow, Error};
use monero_oxide::transaction::{Pruned, Transaction};
use tower::{Service, ServiceExt};

use cuprate_helper::cast::usize_to_u64;
use cuprate_rpc_types::misc::{SpentKeyImageInfo, TxInfo};
use cuprate_txpool::{
    service::{
        interface::{TxpoolReadRequest, TxpoolReadResponse},
        TxpoolReadHandle,
    },
    TxEntry,
};
use cuprate_types::{
    rpc::{PoolTxInfo, TxpoolStats},
    TxInPool,
};
// FIXME: use `anyhow::Error` over `tower::BoxError` in txpool.

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

/// [`TxpoolReadRequest::Size`]
pub(crate) async fn size(
    txpool_read: &mut TxpoolReadHandle,
    include_sensitive_txs: bool,
) -> Result<u64, Error> {
    let TxpoolReadResponse::Size(size) = txpool_read
        .ready()
        .await?
        .call(TxpoolReadRequest::Size {
            include_sensitive_txs,
        })
        .await?
    else {
        unreachable!();
    };

    Ok(usize_to_u64(size))
}

/// [`TxpoolReadRequest::TxsByHash`]/
pub(crate) async fn tx_blobs_by_hash(
    txpool_read: &mut TxpoolReadHandle,
    tx_hashes: &[[u8; 32]],
    prune: bool,
) -> Result<Vec<PoolTxInfo>, Error> {
    let TxpoolReadResponse::TxsByHash(txs) = txpool_read
        .ready()
        .await?
        .call(TxpoolReadRequest::TxsByHash {
            tx_hashes: tx_hashes.to_vec(),
            // TODO: allow sending private txs on restricted.
            include_sensitive_txs: false,
        })
        .await?
    else {
        unreachable!()
    };

    txs.into_iter()
        .map(|t| {
            let mut tx_blob = t.tx_blob;

            if prune {
                // Instead of reading then writing the pruned part to get the pruned blob
                // we can read and then just use the number of bytes read.
                let mut slice = tx_blob.as_slice();
                Transaction::<Pruned>::read(&mut slice)
                    .map_err(|e| anyhow!("failed to parse pool tx blob: {e}"))?;
                let pruned_len = tx_blob.len() - slice.len();
                tx_blob.truncate(pruned_len);
            }

            Ok(PoolTxInfo {
                tx_hash: t.tx_hash,
                tx_blob,
                double_spend_seen: t.double_spend_seen,
            })
        })
        .collect()
}

/// Query the txpool manager for public-pool transactions added/removed since `since`.
pub(crate) async fn pool_info_since(
    txpool_manager: &crate::txpool::TxpoolManagerHandle,
    since: u64,
) -> Result<crate::txpool::PoolInfoSinceResponse, Error> {
    let (response_tx, response_rx) = tokio::sync::oneshot::channel();

    txpool_manager
        .command_tx
        .send(crate::txpool::TxpoolManagerCommand::PoolInfoSince { since, response_tx })
        .await
        .map_err(|_| anyhow!("txpool manager stopped"))?;

    response_rx
        .await
        .map_err(|_| anyhow!("txpool manager stopped"))
}

/// [`TxpoolReadRequest::TxsByHash`]
pub(crate) async fn txs_by_hash(
    txpool_read: &mut TxpoolReadHandle,
    tx_hashes: Vec<[u8; 32]>,
    include_sensitive_txs: bool,
) -> Result<Vec<TxInPool>, Error> {
    let TxpoolReadResponse::TxsByHash(txs_in_pool) = txpool_read
        .ready()
        .await?
        .call(TxpoolReadRequest::TxsByHash {
            tx_hashes,
            include_sensitive_txs,
        })
        .await?
    else {
        unreachable!();
    };

    Ok(txs_in_pool)
}

/// [`TxpoolReadRequest::KeyImagesSpentVec`]
pub(crate) async fn key_images_spent_vec(
    txpool_read: &mut TxpoolReadHandle,
    key_images: Vec<[u8; 32]>,
    include_sensitive_txs: bool,
) -> Result<Vec<bool>, Error> {
    let TxpoolReadResponse::KeyImagesSpentVec(status) = txpool_read
        .ready()
        .await?
        .call(TxpoolReadRequest::KeyImagesSpentVec {
            key_images,
            include_sensitive_txs,
        })
        .await?
    else {
        unreachable!();
    };

    Ok(status)
}

/// [`TxpoolReadRequest::Pool`]
pub(crate) async fn pool(
    txpool_read: &mut TxpoolReadHandle,
    include_sensitive_txs: bool,
) -> Result<(Vec<TxInfo>, Vec<SpentKeyImageInfo>), Error> {
    let TxpoolReadResponse::Pool {
        txs,
        spent_key_images,
    } = txpool_read
        .ready()
        .await?
        .call(TxpoolReadRequest::Pool {
            include_sensitive_txs,
        })
        .await?
    else {
        unreachable!();
    };

    let txs = txs.into_iter().map(Into::into).collect();
    let spent_key_images = spent_key_images.into_iter().map(Into::into).collect();

    Ok((txs, spent_key_images))
}

/// [`TxpoolReadRequest::PoolStats`]
pub(crate) async fn pool_stats(
    txpool_read: &mut TxpoolReadHandle,
    include_sensitive_txs: bool,
) -> Result<TxpoolStats, Error> {
    let TxpoolReadResponse::PoolStats(txpool_stats) = txpool_read
        .ready()
        .await?
        .call(TxpoolReadRequest::PoolStats {
            include_sensitive_txs,
        })
        .await?
    else {
        unreachable!();
    };

    Ok(txpool_stats)
}

/// [`TxpoolReadRequest::AllHashes`]
pub(crate) async fn all_hashes(
    txpool_read: &mut TxpoolReadHandle,
    include_sensitive_txs: bool,
) -> Result<Vec<[u8; 32]>, Error> {
    let TxpoolReadResponse::AllHashes(hashes) = txpool_read
        .ready()
        .await?
        .call(TxpoolReadRequest::AllHashes {
            include_sensitive_txs,
        })
        .await?
    else {
        unreachable!();
    };

    Ok(hashes)
}

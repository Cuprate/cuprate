//! Functions to send [`TxpoolReadRequest`]s.

use std::{collections::HashSet, convert::Infallible, num::NonZero};

use anyhow::{anyhow, Error};
use monero_serai::transaction::Transaction;
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
    rpc::{PoolInfo, PoolInfoFull, PoolInfoIncremental, PoolTxInfo, TxpoolStats},
    TxInPool, TxRelayChecks,
};

// FIXME: use `anyhow::Error` over `tower::BoxError` in txpool.

/// [`TxpoolReadRequest::Backlog`]
pub async fn backlog(txpool_read: &mut TxpoolReadHandle) -> Result<Vec<TxEntry>, Error> {
    let TxpoolReadResponse::Backlog(tx_entries) = txpool_read
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(TxpoolReadRequest::Backlog)
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(tx_entries)
}

/// [`TxpoolReadRequest::Size`]
pub async fn size(
    txpool_read: &mut TxpoolReadHandle,
    include_sensitive_txs: bool,
) -> Result<u64, Error> {
    let TxpoolReadResponse::Size(size) = txpool_read
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(TxpoolReadRequest::Size {
            include_sensitive_txs,
        })
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(usize_to_u64(size))
}

/// [`TxpoolReadRequest::PoolInfo`]
pub async fn pool_info(
    txpool_read: &mut TxpoolReadHandle,
    include_sensitive_txs: bool,
    max_tx_count: usize,
    start_time: Option<NonZero<usize>>,
) -> Result<PoolInfo, Error> {
    let TxpoolReadResponse::PoolInfo(pool_info) = txpool_read
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(TxpoolReadRequest::PoolInfo {
            include_sensitive_txs,
            max_tx_count,
            start_time,
        })
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(pool_info)
}

/// [`TxpoolReadRequest::TxsByHash`]
pub async fn txs_by_hash(
    txpool_read: &mut TxpoolReadHandle,
    tx_hashes: Vec<[u8; 32]>,
    include_sensitive_txs: bool,
) -> Result<Vec<TxInPool>, Error> {
    let TxpoolReadResponse::TxsByHash(txs_in_pool) = txpool_read
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(TxpoolReadRequest::TxsByHash {
            tx_hashes,
            include_sensitive_txs,
        })
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(txs_in_pool)
}

/// [`TxpoolReadRequest::KeyImagesSpent`]
pub async fn key_images_spent(
    txpool_read: &mut TxpoolReadHandle,
    key_images: HashSet<[u8; 32]>,
    include_sensitive_txs: bool,
) -> Result<bool, Error> {
    let TxpoolReadResponse::KeyImagesSpent(status) = txpool_read
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(TxpoolReadRequest::KeyImagesSpent {
            key_images,
            include_sensitive_txs,
        })
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(status)
}

/// [`TxpoolReadRequest::KeyImagesSpentVec`]
pub async fn key_images_spent_vec(
    txpool_read: &mut TxpoolReadHandle,
    key_images: Vec<[u8; 32]>,
    include_sensitive_txs: bool,
) -> Result<Vec<bool>, Error> {
    let TxpoolReadResponse::KeyImagesSpent(status) = txpool_read
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(TxpoolReadRequest::KeyImagesSpent {
            key_images,
            include_sensitive_txs,
        })
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(status)
}

/// [`TxpoolReadRequest::Pool`]
pub async fn pool(
    txpool_read: &mut TxpoolReadHandle,
    include_sensitive_txs: bool,
) -> Result<(Vec<TxInfo>, Vec<SpentKeyImageInfo>), Error> {
    let TxpoolReadResponse::Pool {
        txs,
        spent_key_images,
    } = txpool_read
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(TxpoolReadRequest::Pool {
            include_sensitive_txs,
        })
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    let txs = txs.into_iter().map(Into::into).collect();
    let spent_key_images = spent_key_images.into_iter().map(Into::into).collect();

    Ok((txs, spent_key_images))
}

/// [`TxpoolReadRequest::PoolStats`]
pub async fn pool_stats(
    txpool_read: &mut TxpoolReadHandle,
    include_sensitive_txs: bool,
) -> Result<TxpoolStats, Error> {
    let TxpoolReadResponse::PoolStats(txpool_stats) = txpool_read
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(TxpoolReadRequest::PoolStats {
            include_sensitive_txs,
        })
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(txpool_stats)
}

/// [`TxpoolReadRequest::AllHashes`]
pub async fn all_hashes(
    txpool_read: &mut TxpoolReadHandle,
    include_sensitive_txs: bool,
) -> Result<Vec<[u8; 32]>, Error> {
    let TxpoolReadResponse::AllHashes(hashes) = txpool_read
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(TxpoolReadRequest::AllHashes {
            include_sensitive_txs,
        })
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(hashes)
}

/// TODO: impl txpool manager.
pub async fn flush(txpool_manager: &mut Infallible, tx_hashes: Vec<[u8; 32]>) -> Result<(), Error> {
    todo!();
    Ok(())
}

/// TODO: impl txpool manager.
pub async fn relay(txpool_manager: &mut Infallible, tx_hashes: Vec<[u8; 32]>) -> Result<(), Error> {
    todo!();
    Ok(())
}

/// TODO: impl txpool manager.
pub async fn check_maybe_relay_local(
    txpool_manager: &mut Infallible,
    tx: Transaction,
    relay: bool,
) -> Result<TxRelayChecks, Error> {
    Ok(todo!())
}

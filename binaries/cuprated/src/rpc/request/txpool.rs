//! Functions to send [`TxpoolReadRequest`]s.

use std::{convert::Infallible, num::NonZero};

use anyhow::{anyhow, Error};
use tower::{Service, ServiceExt};

use cuprate_helper::cast::usize_to_u64;
use cuprate_txpool::{
    service::{
        interface::{TxpoolReadRequest, TxpoolReadResponse},
        TxpoolReadHandle,
    },
    TxEntry,
};
use cuprate_types::{
    rpc::{PoolInfoFull, PoolInfoIncremental, PoolTxInfo},
    PoolInfo,
};

// FIXME: use `anyhow::Error` over `tower::BoxError` in txpool.

/// [`TxpoolReadRequest::Backlog`]
pub(crate) async fn backlog(txpool_read: &mut TxpoolReadHandle) -> Result<Vec<TxEntry>, Error> {
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
pub(crate) async fn size(
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

/// TODO
pub(crate) async fn pool_info(
    txpool_read: &mut TxpoolReadHandle,
    include_sensitive_txs: bool,
    max_tx_count: usize,
    start_time: Option<NonZero<usize>>,
) -> Result<Vec<PoolInfo>, Error> {
    let TxpoolReadResponse::PoolInfo(vec) = txpool_read
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

    Ok(vec)
}

/// TODO
pub(crate) async fn flush(
    txpool_manager: &mut Infallible,
    tx_hashes: Vec<[u8; 32]>,
) -> Result<(), Error> {
    todo!();
    Ok(())
}

/// TODO
pub(crate) async fn relay(
    txpool_manager: &mut Infallible,
    tx_hashes: Vec<[u8; 32]>,
) -> Result<(), Error> {
    todo!();
    Ok(())
}

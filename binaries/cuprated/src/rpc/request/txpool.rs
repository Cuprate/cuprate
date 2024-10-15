//! Functions for [`TxpoolReadRequest`].

use std::convert::Infallible;

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
pub(crate) async fn size(txpool_read: &mut TxpoolReadHandle) -> Result<u64, Error> {
    let TxpoolReadResponse::Size(size) = txpool_read
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(TxpoolReadRequest::Size)
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(usize_to_u64(size))
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

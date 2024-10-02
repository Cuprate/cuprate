//! Functions for TODO: doc enum message.

use std::convert::Infallible;

use anyhow::Error;
use tower::{Service, ServiceExt};

use cuprate_helper::cast::usize_to_u64;
use cuprate_txpool::service::{
    interface::{TxpoolReadRequest, TxpoolReadResponse},
    TxpoolReadHandle,
};

/// [`TxpoolReadRequest::Backlog`]
pub(super) async fn backlog(txpool_read: &mut TxpoolReadHandle) -> Result<Vec<Infallible>, Error> {
    let TxpoolReadResponse::Backlog(backlog) = txpool_read
        .ready()
        .await
        .expect("TODO")
        .call(TxpoolReadRequest::Backlog)
        .await
        .expect("TODO")
    else {
        unreachable!();
    };

    Ok(backlog)
}

/// [`TxpoolReadRequest::Size`]
pub(super) async fn size(txpool_read: &mut TxpoolReadHandle) -> Result<u64, Error> {
    let TxpoolReadResponse::Size(size) = txpool_read
        .ready()
        .await
        .expect("TODO")
        .call(TxpoolReadRequest::Size)
        .await
        .expect("TODO")
    else {
        unreachable!();
    };

    Ok(usize_to_u64(size))
}

/// TODO
#[expect(clippy::needless_pass_by_ref_mut, reason = "TODO: remove after impl")]
pub(super) async fn flush(
    txpool_read: &mut TxpoolReadHandle,
    tx_hashes: Vec<[u8; 32]>,
) -> Result<(), Error> {
    todo!();
    Ok(())
}

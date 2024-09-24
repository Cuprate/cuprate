//! Functions for TODO: doc enum message.

use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    sync::Arc,
};

use anyhow::{anyhow, Error};
use cuprate_database_service::DatabaseReadService;
use futures::StreamExt;
use monero_serai::block::Block;
use tower::{Service, ServiceExt};

use cuprate_consensus::BlockchainResponse;
use cuprate_helper::{
    cast::{u64_to_usize, usize_to_u64},
    map::split_u128_into_low_high_bits,
};
use cuprate_txpool::service::{
    interface::{TxpoolReadRequest, TxpoolReadResponse},
    TxpoolReadHandle,
};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainWriteRequest},
    Chain, ExtendedBlockHeader, HardFork, OutputOnChain, VerifiedBlockInformation,
};

use crate::rpc::{CupratedRpcHandler, CupratedRpcHandlerState};

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

// [`::Flush`]
#[expect(clippy::needless_pass_by_ref_mut, reason = "TODO: remove after impl")]
pub(super) async fn flush(
    txpool_read: &mut TxpoolReadHandle,
    tx_hashes: Vec<[u8; 32]>,
) -> Result<(), Error> {
    todo!();
    Ok(())
}

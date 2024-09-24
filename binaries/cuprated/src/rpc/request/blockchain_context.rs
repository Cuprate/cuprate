//! Functions for [`BlockChainContextRequest`] and [`BlockChainContextResponse`].

use std::{
    collections::{HashMap, HashSet},
    convert::Infallible,
    ops::Range,
    sync::Arc,
};

use anyhow::{anyhow, Error};
use futures::StreamExt;
use monero_serai::block::Block;
use randomx_rs::RandomXVM;
use tower::{Service, ServiceExt};

use cuprate_consensus::context::{
    BlockChainContext, BlockChainContextRequest, BlockChainContextResponse,
    BlockChainContextService,
};
use cuprate_helper::{
    cast::{u64_to_usize, usize_to_u64},
    map::split_u128_into_low_high_bits,
};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse, BlockchainWriteRequest},
    Chain, ExtendedBlockHeader, HardFork, OutputOnChain, VerifiedBlockInformation,
};

use crate::rpc::CupratedRpcHandlerState;

/// [`BlockChainContextRequest::Context`].
pub(super) async fn context(
    service: &mut BlockChainContextService,
    height: u64,
) -> Result<BlockChainContext, Error> {
    let BlockChainContextResponse::Context(context) = service
        .ready()
        .await
        .expect("TODO")
        .call(BlockChainContextRequest::Context)
        .await
        .expect("TODO")
    else {
        unreachable!();
    };

    Ok(context)
}

/// [`BlockChainContextRequest::HardForkInfo`].
pub(super) async fn hard_fork_info(
    service: &mut BlockChainContextService,
    hard_fork: HardFork,
) -> Result<Infallible, Error> {
    let BlockChainContextResponse::HardForkInfo(hf_info) = service
        .ready()
        .await
        .expect("TODO")
        .call(BlockChainContextRequest::HardForkInfo(hard_fork))
        .await
        .expect("TODO")
    else {
        unreachable!();
    };

    Ok(hf_info)
}

/// [`BlockChainContextRequest::FeeEstimate`].
pub(super) async fn fee_estimate(
    service: &mut BlockChainContextService,
    grace_blocks: u64,
) -> Result<Infallible, Error> {
    let BlockChainContextResponse::FeeEstimate(hf_info) = service
        .ready()
        .await
        .expect("TODO")
        .call(BlockChainContextRequest::FeeEstimate { grace_blocks })
        .await
        .expect("TODO")
    else {
        unreachable!();
    };

    Ok(hf_info)
}

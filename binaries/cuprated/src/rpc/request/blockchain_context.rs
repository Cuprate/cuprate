//! Functions for [`BlockChainContextRequest`] and [`BlockChainContextResponse`].

use std::convert::Infallible;

use anyhow::Error;
use tower::{Service, ServiceExt};

use cuprate_consensus::context::{
    BlockChainContext, BlockChainContextRequest, BlockChainContextResponse,
    BlockChainContextService,
};
use cuprate_types::HardFork;

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

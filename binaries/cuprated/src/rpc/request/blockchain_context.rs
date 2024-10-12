//! Functions for [`BlockChainContextRequest`] and [`BlockChainContextResponse`].

use std::convert::Infallible;

use anyhow::{anyhow, Error};
use tower::{Service, ServiceExt};

use cuprate_consensus::context::{
    BlockChainContext, BlockChainContextRequest, BlockChainContextResponse,
    BlockChainContextService,
};
use cuprate_types::{FeeEstimate, HardFork, HardForkInfo};

// FIXME: use `anyhow::Error` over `tower::BoxError` in blockchain context.

/// [`BlockChainContextRequest::Context`].
pub(crate) async fn context(
    service: &mut BlockChainContextService,
) -> Result<BlockChainContext, Error> {
    let BlockChainContextResponse::Context(context) = service
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(BlockChainContextRequest::Context)
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(context)
}

/// [`BlockChainContextRequest::HardForkInfo`].
pub(crate) async fn hard_fork_info(
    service: &mut BlockChainContextService,
    hard_fork: HardFork,
) -> Result<HardForkInfo, Error> {
    let BlockChainContextResponse::HardForkInfo(hf_info) = service
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(BlockChainContextRequest::HardForkInfo(hard_fork))
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(hf_info)
}

/// [`BlockChainContextRequest::FeeEstimate`].
pub(crate) async fn fee_estimate(
    service: &mut BlockChainContextService,
    grace_blocks: u64,
) -> Result<FeeEstimate, Error> {
    let BlockChainContextResponse::FeeEstimate(fee) = service
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(BlockChainContextRequest::FeeEstimate { grace_blocks })
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(fee)
}

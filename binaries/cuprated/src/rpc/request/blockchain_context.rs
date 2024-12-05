//! Functions to send [`BlockChainContextRequest`]s.

use anyhow::{anyhow, Error};
use monero_serai::block::Block;
use tower::{Service, ServiceExt};

use cuprate_consensus_context::{
    BlockChainContext, BlockChainContextRequest, BlockChainContextResponse,
    BlockChainContextService,
};
use cuprate_types::{
    rpc::{FeeEstimate, HardForkInfo},
    HardFork,
};

// FIXME: use `anyhow::Error` over `tower::BoxError` in blockchain context.

/// [`BlockChainContextRequest::Context`].
pub(crate) async fn context(
    blockchain_context: &mut BlockChainContextService,
) -> Result<BlockChainContext, Error> {
    let BlockChainContextResponse::Context(context) = blockchain_context
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
    blockchain_context: &mut BlockChainContextService,
    hard_fork: HardFork,
) -> Result<HardForkInfo, Error> {
    let BlockChainContextResponse::HardForkInfo(hf_info) = blockchain_context
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
    blockchain_context: &mut BlockChainContextService,
    grace_blocks: u64,
) -> Result<FeeEstimate, Error> {
    let BlockChainContextResponse::FeeEstimate(fee) = blockchain_context
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

/// [`BlockChainContextRequest::CalculatePow`]
pub(crate) async fn calculate_pow(
    blockchain_context: &mut BlockChainContextService,
    hardfork: HardFork,
    block: Block,
    seed_hash: [u8; 32],
) -> Result<[u8; 32], Error> {
    let Some(height) = block.number() else {
        return Err(anyhow!("Block is missing height"));
    };

    let block = Box::new(block);

    let BlockChainContextResponse::CalculatePow(hash) = blockchain_context
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(BlockChainContextRequest::CalculatePow {
            hardfork,
            height,
            block,
            seed_hash,
        })
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(hash)
}

/// [`BlockChainContextRequest::BatchGetDifficulties`]
pub(crate) async fn batch_get_difficulties(
    blockchain_context: &mut BlockChainContextService,
    difficulties: Vec<(u64, HardFork)>,
) -> Result<Vec<u128>, Error> {
    let BlockChainContextResponse::BatchDifficulties(resp) = blockchain_context
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(BlockChainContextRequest::BatchGetDifficulties(difficulties))
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(resp)
}

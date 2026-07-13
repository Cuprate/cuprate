//! Functions to send [`BlockChainContextRequest`]s.

use std::num::NonZero;

use anyhow::{anyhow, Error};
use monero_oxide::block::Block;
use tower::{Service, ServiceExt};

use cuprate_consensus_context::{
    BlockChainContextRequest, BlockChainContextResponse, BlockchainContext,
    BlockchainContextService,
};
use cuprate_types::{
    rpc::{FeeEstimate, HardForkInfo, OutputDistributionData},
    HardFork,
};

// FIXME: use `anyhow::Error` over `tower::BoxError` in blockchain context.

/// [`BlockChainContextRequest::HardForkInfos`].
pub(crate) async fn hard_fork_infos(
    blockchain_context: &mut BlockchainContextService,
) -> Result<Vec<HardForkInfo>, Error> {
    let BlockChainContextResponse::HardForkInfos(hf_info) = blockchain_context
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(BlockChainContextRequest::HardForkInfos)
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(hf_info)
}

/// [`BlockChainContextRequest::FeeEstimate`].
pub(crate) async fn fee_estimate(
    blockchain_context: &mut BlockchainContextService,
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

/// [`BlockChainContextRequest::RctOutputDistribution`]
pub(crate) async fn rct_output_distribution(
    blockchain_context: &mut BlockchainContextService,
    from_height: u64,
    to_height: Option<NonZero<u64>>,
    cumulative: bool,
) -> Result<OutputDistributionData, Error> {
    let BlockChainContextResponse::RctOutputDistribution(data) = blockchain_context
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(BlockChainContextRequest::RctOutputDistribution {
            from_height,
            to_height,
            cumulative,
        })
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(data)
}

/// [`BlockChainContextRequest::CalculatePow`]
pub(crate) async fn calculate_pow(
    blockchain_context: &mut BlockchainContextService,
    hardfork: HardFork,
    block: Block,
    seed_hash: [u8; 32],
) -> Result<[u8; 32], Error> {
    let height = block.number();

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
pub async fn batch_get_difficulties(
    blockchain_context: &mut BlockchainContextService,
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

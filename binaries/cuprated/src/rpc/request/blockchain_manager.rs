//! Functions for [`BlockchainManagerRequest`] & [`BlockchainManagerResponse`].

use anyhow::Error;
use cuprate_types::{AddAuxPow, AuxPow, HardFork};
use monero_serai::block::Block;
use tower::{Service, ServiceExt};

use cuprate_helper::cast::{u64_to_usize, usize_to_u64};
use cuprate_pruning::PruningSeed;

use crate::rpc::handler::{
    BlockchainManagerHandle, BlockchainManagerRequest, BlockchainManagerResponse,
};

/// [`BlockchainManagerRequest::PopBlocks`]
pub(crate) async fn pop_blocks(
    blockchain_manager: &mut BlockchainManagerHandle,
    amount: u64,
) -> Result<u64, Error> {
    let BlockchainManagerResponse::PopBlocks { new_height } = blockchain_manager
        .ready()
        .await?
        .call(BlockchainManagerRequest::PopBlocks {
            amount: u64_to_usize(amount),
        })
        .await?
    else {
        unreachable!();
    };

    Ok(usize_to_u64(new_height))
}

/// [`BlockchainManagerRequest::Prune`]
pub(crate) async fn prune(
    blockchain_manager: &mut BlockchainManagerHandle,
) -> Result<PruningSeed, Error> {
    let BlockchainManagerResponse::Prune(seed) = blockchain_manager
        .ready()
        .await?
        .call(BlockchainManagerRequest::Prune)
        .await?
    else {
        unreachable!();
    };

    Ok(seed)
}

/// [`BlockchainManagerRequest::Pruned`]
pub(crate) async fn pruned(
    blockchain_manager: &mut BlockchainManagerHandle,
) -> Result<bool, Error> {
    let BlockchainManagerResponse::Pruned(pruned) = blockchain_manager
        .ready()
        .await?
        .call(BlockchainManagerRequest::Pruned)
        .await?
    else {
        unreachable!();
    };

    Ok(pruned)
}

/// [`BlockchainManagerRequest::RelayBlock`]
pub(crate) async fn relay_block(
    blockchain_manager: &mut BlockchainManagerHandle,
    block: Block,
) -> Result<(), Error> {
    let BlockchainManagerResponse::Ok = blockchain_manager
        .ready()
        .await?
        .call(BlockchainManagerRequest::RelayBlock(block))
        .await?
    else {
        unreachable!();
    };

    Ok(())
}

/// [`BlockchainManagerRequest::Syncing`]
pub(crate) async fn syncing(
    blockchain_manager: &mut BlockchainManagerHandle,
) -> Result<bool, Error> {
    let BlockchainManagerResponse::Syncing(syncing) = blockchain_manager
        .ready()
        .await?
        .call(BlockchainManagerRequest::Syncing)
        .await?
    else {
        unreachable!();
    };

    Ok(syncing)
}

/// [`BlockchainManagerRequest::Synced`]
pub(crate) async fn synced(
    blockchain_manager: &mut BlockchainManagerHandle,
) -> Result<bool, Error> {
    let BlockchainManagerResponse::Synced(syncing) = blockchain_manager
        .ready()
        .await?
        .call(BlockchainManagerRequest::Synced)
        .await?
    else {
        unreachable!();
    };

    Ok(syncing)
}

/// [`BlockchainManagerRequest::Target`]
pub(crate) async fn target(
    blockchain_manager: &mut BlockchainManagerHandle,
) -> Result<std::time::Duration, Error> {
    let BlockchainManagerResponse::Target(target) = blockchain_manager
        .ready()
        .await?
        .call(BlockchainManagerRequest::Target)
        .await?
    else {
        unreachable!();
    };

    Ok(target)
}

/// [`BlockchainManagerRequest::TargetHeight`]
pub(crate) async fn target_height(
    blockchain_manager: &mut BlockchainManagerHandle,
) -> Result<u64, Error> {
    let BlockchainManagerResponse::TargetHeight { height } = blockchain_manager
        .ready()
        .await?
        .call(BlockchainManagerRequest::TargetHeight)
        .await?
    else {
        unreachable!();
    };

    Ok(usize_to_u64(height))
}

/// [`BlockchainManagerRequest::CalculatePow`]
pub(crate) async fn calculate_pow(
    blockchain_manager: &mut BlockchainManagerHandle,
    hardfork: HardFork,
    height: u64,
    block: Block,
    seed_hash: [u8; 32],
) -> Result<[u8; 32], Error> {
    let BlockchainManagerResponse::CalculatePow(hash) = blockchain_manager
        .ready()
        .await?
        .call(BlockchainManagerRequest::CalculatePow {
            hardfork,
            height: u64_to_usize(height),
            block,
            seed_hash,
        })
        .await?
    else {
        unreachable!();
    };

    Ok(hash)
}

/// [`BlockchainManagerRequest::AddAuxPow`]
pub(crate) async fn add_aux_pow(
    blockchain_manager: &mut BlockchainManagerHandle,
    blocktemplate_blob: Vec<u8>,
    aux_pow: Vec<AuxPow>,
) -> Result<AddAuxPow, Error> {
    let BlockchainManagerResponse::AddAuxPow(response) = blockchain_manager
        .ready()
        .await?
        .call(BlockchainManagerRequest::AddAuxPow {
            blocktemplate_blob,
            aux_pow,
        })
        .await?
    else {
        unreachable!();
    };

    Ok(response)
}

/// [`BlockchainManagerRequest::GenerateBlocks`]
pub(crate) async fn generate_blocks(
    blockchain_manager: &mut BlockchainManagerHandle,
    amount_of_blocks: u64,
    prev_block: [u8; 32],
    starting_nonce: u32,
    wallet_address: String,
) -> Result<(Vec<[u8; 32]>, u64), Error> {
    let BlockchainManagerResponse::GenerateBlocks { blocks, height } = blockchain_manager
        .ready()
        .await?
        .call(BlockchainManagerRequest::GenerateBlocks {
            amount_of_blocks,
            prev_block,
            starting_nonce,
            wallet_address,
        })
        .await?
    else {
        unreachable!();
    };

    Ok((blocks, usize_to_u64(height)))
}

/// [`BlockchainManagerRequest::Overview`]
pub(crate) async fn overview(
    blockchain_manager: &mut BlockchainManagerHandle,
    height: u64,
) -> Result<String, Error> {
    let BlockchainManagerResponse::Overview(overview) = blockchain_manager
        .ready()
        .await?
        .call(BlockchainManagerRequest::Overview {
            height: u64_to_usize(height),
        })
        .await?
    else {
        unreachable!();
    };

    Ok(overview)
}

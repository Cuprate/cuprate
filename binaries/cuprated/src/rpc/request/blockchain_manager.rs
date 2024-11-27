//! Functions to send [`BlockchainManagerRequest`]s.

use anyhow::Error;
use monero_serai::block::Block;
use tower::{Service, ServiceExt};

use cuprate_helper::cast::{u64_to_usize, usize_to_u64};
use cuprate_p2p_core::{types::ConnectionId, NetworkZone};
use cuprate_pruning::PruningSeed;
use cuprate_rpc_types::misc::Span;

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
    block: Box<Block>,
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

// [`BlockchainManagerRequest::Spans`]
pub(crate) async fn spans<Z: NetworkZone>(
    blockchain_manager: &mut BlockchainManagerHandle,
) -> Result<Vec<Span>, Error> {
    // let BlockchainManagerResponse::Spans(vec) = blockchain_manager
    //     .ready()
    //     .await?
    //     .call(BlockchainManagerRequest::Spans)
    //     .await?
    // else {
    //     unreachable!();
    // };

    let vec: Vec<cuprate_p2p_core::types::Span<Z::Addr>> =
        todo!("waiting on blockchain downloader/syncer: <https://github.com/Cuprate/cuprate/pull/320#discussion_r1811089758>");

    // FIXME: impl this map somewhere instead of inline.
    let vec = vec
        .into_iter()
        .map(|span| Span {
            connection_id: String::from(ConnectionId::DEFAULT_STR),
            nblocks: span.nblocks,
            rate: span.rate,
            remote_address: span.remote_address.to_string(),
            size: span.size,
            speed: span.speed,
            start_block_height: span.start_block_height,
        })
        .collect();

    Ok(vec)
}

/// [`BlockchainManagerRequest::NextNeededPruningSeed`]
pub(crate) async fn next_needed_pruning_seed(
    blockchain_manager: &mut BlockchainManagerHandle,
) -> Result<PruningSeed, Error> {
    let BlockchainManagerResponse::NextNeededPruningSeed(seed) = blockchain_manager
        .ready()
        .await?
        .call(BlockchainManagerRequest::NextNeededPruningSeed)
        .await?
    else {
        unreachable!();
    };

    Ok(seed)
}

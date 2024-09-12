//! These are convenience functions that make
//! sending [`BlockchainReadRequest`] less verbose.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::{anyhow, Error};
use futures::StreamExt;
use monero_serai::block::Block;
use tower::{Service, ServiceExt};

use cuprate_consensus::BlockchainResponse;
use cuprate_helper::{
    cast::{u64_to_usize, usize_to_u64},
    map::split_u128_into_low_high_bits,
};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainWriteRequest},
    Chain, ExtendedBlockHeader, OutputOnChain, VerifiedBlockInformation,
};

use crate::rpc::{CupratedRpcHandlerState, RESTRICTED_BLOCK_COUNT, RESTRICTED_BLOCK_HEADER_RANGE};

/// [`BlockchainResponse::ChainHeight`].
pub(super) async fn chain_height(
    state: &mut CupratedRpcHandlerState,
) -> Result<(u64, [u8; 32]), Error> {
    let BlockchainResponse::ChainHeight(height, hash) = state
        .blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::ChainHeight)
        .await?
    else {
        unreachable!();
    };

    Ok((usize_to_u64(height), hash))
}

/// [`BlockchainResponse::Block`].
pub(super) async fn block(
    state: &mut CupratedRpcHandlerState,
    height: u64,
) -> Result<Block, Error> {
    let BlockchainResponse::Block(block) = state
        .blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::Block(u64_to_usize(height)))
        .await?
    else {
        unreachable!();
    };

    Ok(block)
}

/// [`BlockchainResponse::BlockByHash`].
pub(super) async fn block_by_hash(
    state: &mut CupratedRpcHandlerState,
    hash: [u8; 32],
) -> Result<Block, Error> {
    let BlockchainResponse::BlockByHash(block) = state
        .blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::BlockByHash(hash))
        .await?
    else {
        unreachable!();
    };

    Ok(block)
}

/// [`BlockchainResponse::BlockExtendedHeader`].
pub(super) async fn block_extended_header(
    state: &mut CupratedRpcHandlerState,
    height: u64,
) -> Result<ExtendedBlockHeader, Error> {
    let BlockchainResponse::BlockExtendedHeader(header) = state
        .blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::BlockExtendedHeader(u64_to_usize(
            height,
        )))
        .await?
    else {
        unreachable!();
    };

    Ok(header)
}

/// [`BlockchainResponse::BlockExtendedHeaderByHash`].
pub(super) async fn block_extended_header_by_hash(
    state: &mut CupratedRpcHandlerState,
    hash: [u8; 32],
) -> Result<ExtendedBlockHeader, Error> {
    let BlockchainResponse::BlockExtendedHeaderByHash(header) = state
        .blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::BlockExtendedHeaderByHash(hash))
        .await?
    else {
        unreachable!();
    };

    Ok(header)
}

/// [`BlockchainResponse::TopBlockFull`].
pub(super) async fn top_block_full(
    state: &mut CupratedRpcHandlerState,
) -> Result<(Block, ExtendedBlockHeader), Error> {
    let BlockchainResponse::TopBlockFull(block, header) = state
        .blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::TopBlockFull)
        .await?
    else {
        unreachable!();
    };

    Ok((block, header))
}

/// [`BlockchainResponse::BlockHash`] with [`Chain::Main`].
pub(super) async fn block_hash(
    state: &mut CupratedRpcHandlerState,
    height: u64,
) -> Result<[u8; 32], Error> {
    let BlockchainResponse::BlockHash(hash) = state
        .blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::BlockHash(
            u64_to_usize(height),
            Chain::Main,
        ))
        .await?
    else {
        unreachable!();
    };

    Ok(hash)
}

/// [`BlockchainResponse::KeyImageSpent`]
pub(super) async fn key_image_spent(
    state: &mut CupratedRpcHandlerState,
    key_image: [u8; 32],
) -> Result<bool, Error> {
    let BlockchainResponse::KeyImageSpent(is_spent) = state
        .blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::KeyImageSpent(key_image))
        .await?
    else {
        unreachable!();
    };

    Ok(is_spent)
}

/// [`BlockchainResponse::Outputs`]
pub(super) async fn outputs(
    state: &mut CupratedRpcHandlerState,
    outputs: HashMap<u64, HashSet<u64>>,
) -> Result<HashMap<u64, HashMap<u64, OutputOnChain>>, Error> {
    let BlockchainResponse::Outputs(outputs) = state
        .blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::Outputs(outputs))
        .await?
    else {
        unreachable!();
    };

    Ok(outputs)
}

/// [`BlockchainResponse::PopBlocks`]
pub(super) async fn pop_blocks(
    state: &mut CupratedRpcHandlerState,
    nblocks: u64,
) -> Result<u64, Error> {
    let BlockchainResponse::PopBlocks(height) = state
        .blockchain_write
        .ready()
        .await?
        .call(BlockchainWriteRequest::PopBlocks(nblocks))
        .await?
    else {
        unreachable!();
    };

    Ok(usize_to_u64(height))
}

/// [`BlockchainResponse::FindFirstUnknown`]
pub(super) async fn find_first_unknown(
    state: &mut CupratedRpcHandlerState,
    hashes: Vec<[u8; 32]>,
) -> Result<Option<(usize, u64)>, Error> {
    let BlockchainResponse::FindFirstUnknown(resp) = state
        .blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::FindFirstUnknown(hashes))
        .await?
    else {
        unreachable!();
    };

    Ok(resp.map(|(index, height)| (index, usize_to_u64(height))))
}

/// [`BlockchainResponse::CumulativeBlockWeightLimit`]
pub(super) async fn cumulative_block_weight_limit(
    state: &mut CupratedRpcHandlerState,
) -> Result<usize, Error> {
    let BlockchainResponse::CumulativeBlockWeightLimit(limit) = state
        .blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::CumulativeBlockWeightLimit)
        .await?
    else {
        unreachable!();
    };

    Ok(limit)
}

// FindBlock([u8; 32]),
// FilterUnknownHashes(HashSet<[u8; 32]>),
// BlockExtendedHeaderInRange(Range<usize>, Chain),
// ChainHeight,
// GeneratedCoins(usize),
// Outputs(HashMap<u64, HashSet<u64>>),
// NumberOutputsWithAmount(Vec<u64>),
// KeyImagesSpent(HashSet<[u8; 32]>),
// CompactChainHistory,
// FindFirstUnknown(Vec<[u8; 32]>),

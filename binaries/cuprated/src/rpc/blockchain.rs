//! These are convenience functions that make
//! sending [`BlockchainReadRequest`] less verbose.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::{anyhow, Error};
use futures::StreamExt;
use tower::{Service, ServiceExt};

use cuprate_consensus::BlockchainResponse;
use cuprate_helper::{
    cast::{u64_to_usize, usize_to_u64},
    map::split_u128_into_low_high_bits,
};
use cuprate_types::{
    blockchain::BlockchainReadRequest, Chain, ExtendedBlockHeader, OutputOnChain,
    VerifiedBlockInformation,
};

use crate::rpc::{CupratedRpcHandlerState, RESTRICTED_BLOCK_COUNT, RESTRICTED_BLOCK_HEADER_RANGE};

/// [`BlockchainResponse::ChainHeight`].
pub(super) async fn chain_height(
    state: &mut CupratedRpcHandlerState,
) -> Result<(u64, [u8; 32]), Error> {
    let BlockchainResponse::ChainHeight(height, hash) = state
        .blockchain
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
) -> Result<VerifiedBlockInformation, Error> {
    let BlockchainResponse::Block(block) = state
        .blockchain
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
) -> Result<VerifiedBlockInformation, Error> {
    let BlockchainResponse::BlockByHash(block) = state
        .blockchain
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
        .blockchain
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
        .blockchain
        .ready()
        .await?
        .call(BlockchainReadRequest::BlockExtendedHeaderByHash(hash))
        .await?
    else {
        unreachable!();
    };

    Ok(header)
}

/// [`BlockchainResponse::BlockHash`] with [`Chain::Main`].
pub(super) async fn block_hash(
    state: &mut CupratedRpcHandlerState,
    height: u64,
) -> Result<[u8; 32], Error> {
    let BlockchainResponse::BlockHash(hash) = state
        .blockchain
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
        .blockchain
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
        .blockchain
        .ready()
        .await?
        .call(BlockchainReadRequest::Outputs(outputs))
        .await?
    else {
        unreachable!();
    };

    Ok(outputs)
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

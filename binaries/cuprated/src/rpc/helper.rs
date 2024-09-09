//! These are internal helper functions used by the actual RPC handlers.
//!
//! Many of the handlers have bodies with only small differences,
//! the identical code is extracted and reused here in these functions.
//!
//! These build on-top of [`crate::rpc::blockchain`] functions.

use std::sync::Arc;

use anyhow::{anyhow, Error};
use cuprate_rpc_types::misc::BlockHeader;
use futures::StreamExt;
use tower::{Service, ServiceExt};

use cuprate_consensus::BlockchainResponse;
use cuprate_helper::{
    cast::{u64_to_usize, usize_to_u64},
    map::split_u128_into_low_high_bits,
};
use cuprate_types::{
    blockchain::BlockchainReadRequest, Chain, ExtendedBlockHeader, VerifiedBlockInformation,
};

use crate::{
    rpc::blockchain,
    rpc::{CupratedRpcHandlerState, RESTRICTED_BLOCK_COUNT, RESTRICTED_BLOCK_HEADER_RANGE},
};

/// Get a [`VerifiedBlockInformation`] and map it to a [`BlockHeader`].
pub(super) async fn block_header(
    state: &mut CupratedRpcHandlerState,
    height: u64,
    fill_pow_hash: bool,
) -> Result<(VerifiedBlockInformation, BlockHeader), Error> {
    let block = blockchain::block(state, height).await?;
    let mut block_header = BlockHeader::from(&block);
    if !fill_pow_hash {
        block_header.pow_hash = String::new();
    }
    Ok((block, block_header))
}

/// Same as [`block_header`] but with the block's hash.
pub(super) async fn block_header_by_hash(
    state: &mut CupratedRpcHandlerState,
    hash: [u8; 32],
    fill_pow_hash: bool,
) -> Result<(VerifiedBlockInformation, BlockHeader), Error> {
    let block = blockchain::block_by_hash(state, hash).await?;
    let mut block_header = BlockHeader::from(&block);
    if !fill_pow_hash {
        block_header.pow_hash = String::new();
    }
    Ok((block, block_header))
}

/// Check if `height` is greater than the [`top_height`].
///
/// # Errors
/// This returns the [`top_height`] on [`Ok`] and
/// returns [`Error`] if `height` is greater than [`top_height`].
pub(super) async fn check_height(
    state: &mut CupratedRpcHandlerState,
    height: u64,
) -> Result<u64, Error> {
    let (top_height, _) = top_height(state).await?;

    if height > top_height {
        return Err(anyhow!(
            "Requested block height: {height} greater than current top block height: {top_height}",
        ));
    }

    Ok(top_height)
}

/// Parse a hexadecimal [`String`] as a 32-byte hash.
pub(super) fn hex_to_hash(hex: String) -> Result<[u8; 32], Error> {
    let error = || anyhow!("Failed to parse hex representation of hash. Hex = {hex}.");

    let Ok(bytes) = hex::decode(&hex) else {
        return Err(error());
    };

    let Ok(hash) = bytes.try_into() else {
        return Err(error());
    };

    Ok(hash)
}

/// [`BlockchainResponse::ChainHeight`] minus 1.
pub(super) async fn top_height(
    state: &mut CupratedRpcHandlerState,
) -> Result<(u64, [u8; 32]), Error> {
    let (chain_height, hash) = blockchain::chain_height(state).await?;
    let height = chain_height.saturating_sub(1);
    Ok((height, hash))
}

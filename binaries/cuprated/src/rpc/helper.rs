//! These are internal helper functions used by the actual RPC handlers.
//!
//! Many of the handlers have bodies with only small differences,
//! the identical code is extracted and reused here in these functions.
//!
//! These build on-top of [`crate::rpc::request`] functions.

use anyhow::{anyhow, Error};

use cuprate_helper::{
    cast::{u64_to_usize, usize_to_u64},
    map::split_u128_into_low_high_bits,
};
use cuprate_rpc_types::{
    base::{AccessResponseBase, ResponseBase},
    misc::BlockHeader,
};
use cuprate_types::HardFork;

use crate::{
    rpc::request::{blockchain, blockchain_context},
    rpc::CupratedRpcHandler,
};

/// Map some data into a [`BlockHeader`].
///
/// Sort of equivalent to:
/// <https://github.com/monero-project/monero/blob/893916ad091a92e765ce3241b94e706ad012b62a/src/rpc/core_rpc_server.cpp#L2361>.
pub(super) async fn block_header(
    state: &mut CupratedRpcHandler,
    height: u64,
    fill_pow_hash: bool,
) -> Result<BlockHeader, Error> {
    let block = blockchain::block(&mut state.blockchain_read, height).await?;
    let header = blockchain::block_extended_header(&mut state.blockchain_read, height).await?;
    let hardfork = HardFork::from_vote(header.vote);
    let (top_height, _) = top_height(state).await?;

    // TODO: if the request block is not on the main chain,
    // we must get the alt block and this variable will be `true`.
    let orphan_status = false;

    // FIXME: is there a cheaper way to get this?
    let difficulty = blockchain_context::batch_get_difficulties(
        &mut state.blockchain_context,
        vec![(height, hardfork)],
    )
    .await?
    .first()
    .copied()
    .ok_or_else(|| anyhow!("Failed to get block difficulty"))?;

    let pow_hash = if fill_pow_hash {
        let seed_height =
            cuprate_consensus_rules::blocks::randomx_seed_height(u64_to_usize(height));
        let seed_hash = blockchain::block_hash(
            &mut state.blockchain_read,
            height,
            todo!("access to `cuprated`'s Chain"),
        )
        .await?;

        Some(
            blockchain_context::calculate_pow(
                &mut state.blockchain_context,
                hardfork,
                block,
                seed_hash,
            )
            .await?,
        )
    } else {
        None
    };

    let block_weight = usize_to_u64(header.block_weight);
    let depth = top_height.saturating_sub(height);

    let (cumulative_difficulty_top64, cumulative_difficulty) =
        split_u128_into_low_high_bits(header.cumulative_difficulty);
    let (difficulty_top64, difficulty) = split_u128_into_low_high_bits(difficulty);

    let reward = block
        .miner_transaction
        .prefix()
        .outputs
        .iter()
        .map(|o| o.amount.expect("coinbase is transparent"))
        .sum::<u64>();

    Ok(cuprate_types::rpc::BlockHeader {
        block_weight,
        cumulative_difficulty_top64,
        cumulative_difficulty,
        depth,
        difficulty_top64,
        difficulty,
        hash: block.hash(),
        height,
        long_term_weight: usize_to_u64(header.long_term_weight),
        major_version: header.version,
        miner_tx_hash: block.miner_transaction.hash(),
        minor_version: header.vote,
        nonce: block.header.nonce,
        num_txes: usize_to_u64(block.transactions.len()),
        orphan_status,
        pow_hash,
        prev_hash: block.header.previous,
        reward,
        timestamp: block.header.timestamp,
    }
    .into())
}

/// Same as [`block_header`] but with the block's hash.
pub(super) async fn block_header_by_hash(
    state: &mut CupratedRpcHandler,
    hash: [u8; 32],
    fill_pow_hash: bool,
) -> Result<BlockHeader, Error> {
    let (_, height) = blockchain::find_block(&mut state.blockchain_read, hash)
        .await?
        .ok_or_else(|| anyhow!("Block did not exist."))?;

    let block_header = block_header(state, usize_to_u64(height), fill_pow_hash).await?;

    Ok(block_header)
}

/// Check if `height` is greater than the [`top_height`].
///
/// # Errors
/// This returns the [`top_height`] on [`Ok`] and
/// returns [`Error`] if `height` is greater than [`top_height`].
pub(super) async fn check_height(
    state: &mut CupratedRpcHandler,
    height: u64,
) -> Result<u64, Error> {
    let (top_height, _) = top_height(state).await?;

    if height > top_height {
        return Err(anyhow!(
            "Requested block height: {height} greater than top block height: {top_height}",
        ));
    }

    Ok(top_height)
}

/// Parse a hexadecimal [`String`] as a 32-byte hash.
#[expect(clippy::needless_pass_by_value)]
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

/// [`cuprate_types::blockchain::BlockchainResponse::ChainHeight`] minus 1.
pub(super) async fn top_height(state: &mut CupratedRpcHandler) -> Result<(u64, [u8; 32]), Error> {
    let (chain_height, hash) = blockchain::chain_height(&mut state.blockchain_read).await?;
    let height = chain_height.saturating_sub(1);
    Ok((height, hash))
}

/// TODO
pub const fn response_base(is_bootstrap: bool) -> ResponseBase {
    if is_bootstrap {
        ResponseBase::OK_UNTRUSTED
    } else {
        ResponseBase::OK
    }
}

/// TODO
pub const fn access_response_base(is_bootstrap: bool) -> AccessResponseBase {
    if is_bootstrap {
        AccessResponseBase::OK_UNTRUSTED
    } else {
        AccessResponseBase::OK
    }
}

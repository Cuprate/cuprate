//! These are internal helper functions used by the actual RPC handlers.
//!
//! Many of the handlers have bodies with only small differences,
//! the identical code is extracted and reused here in these functions.
//!
//! These build on-top of [`crate::rpc::request`] functions.

use anyhow::{anyhow, Error};
use monero_serai::block::Block;

use cuprate_helper::{cast::usize_to_u64, map::split_u128_into_low_high_bits};
use cuprate_rpc_types::misc::{BlockHeader, KeyImageSpentStatus};
use cuprate_types::ExtendedBlockHeader;

use crate::{rpc::request::blockchain, rpc::CupratedRpcHandler};

fn into_block_header(
    height: u64,
    top_height: u64,
    fill_pow_hash: bool,
    block: Block,
    header: ExtendedBlockHeader,
) -> BlockHeader {
    let block_weight = usize_to_u64(header.block_weight);
    let depth = top_height.saturating_sub(height);
    let (cumulative_difficulty_top64, cumulative_difficulty) =
        split_u128_into_low_high_bits(header.cumulative_difficulty);

    BlockHeader {
        block_size: block_weight,
        block_weight,
        cumulative_difficulty_top64,
        cumulative_difficulty,
        depth,
        difficulty_top64: todo!(),
        difficulty: todo!(),
        hash: hex::encode(block.hash()),
        height,
        long_term_weight: usize_to_u64(header.long_term_weight),
        major_version: header.version.as_u8(),
        miner_tx_hash: hex::encode(block.miner_transaction.hash()),
        minor_version: header.vote,
        nonce: block.header.nonce,
        num_txes: usize_to_u64(block.transactions.len()),
        orphan_status: todo!(),
        pow_hash: if fill_pow_hash {
            todo!()
        } else {
            String::new()
        },
        prev_hash: hex::encode(block.header.previous),
        reward: todo!(),
        timestamp: block.header.timestamp,
        wide_cumulative_difficulty: hex::encode(u128::to_le_bytes(header.cumulative_difficulty)),
        wide_difficulty: todo!(),
    }
}

/// Get a [`VerifiedBlockInformation`] and map it to a [`BlockHeader`].
pub(super) async fn block_header(
    state: &mut CupratedRpcHandler,
    height: u64,
    fill_pow_hash: bool,
) -> Result<BlockHeader, Error> {
    let (top_height, _) = top_height(state).await?;
    let block = blockchain::block(&mut state.blockchain_read, height).await?;
    let header = blockchain::block_extended_header(&mut state.blockchain_read, height).await?;

    let block_header = into_block_header(height, top_height, fill_pow_hash, block, header);

    Ok(block_header)
}

/// Same as [`block_header`] but with the block's hash.
pub(super) async fn block_header_by_hash(
    state: &mut CupratedRpcHandler,
    hash: [u8; 32],
    fill_pow_hash: bool,
) -> Result<BlockHeader, Error> {
    let (top_height, _) = top_height(state).await?;
    let block = blockchain::block_by_hash(&mut state.blockchain_read, hash).await?;
    let header: ExtendedBlockHeader = todo!(); //blockchain::block_extended_header_by_hash(state.blockchain_read, hash).await?;

    let block_header = into_block_header(todo!(), top_height, fill_pow_hash, block, header);

    Ok(block_header)
}

/// TODO
pub(super) async fn top_block_header(
    state: &mut CupratedRpcHandler,
    fill_pow_hash: bool,
) -> Result<BlockHeader, Error> {
    let block: Block = todo!();
    let header: ExtendedBlockHeader = todo!();

    let block_header = into_block_header(todo!(), todo!(), fill_pow_hash, block, header);

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
pub(super) async fn top_height(state: &mut CupratedRpcHandler) -> Result<(u64, [u8; 32]), Error> {
    let (chain_height, hash) = blockchain::chain_height(&mut state.blockchain_read).await?;
    let height = chain_height.saturating_sub(1);
    Ok((height, hash))
}

/// TODO
pub(super) async fn key_image_spent(
    state: &mut CupratedRpcHandler,
    key_image: [u8; 32],
) -> Result<KeyImageSpentStatus, Error> {
    todo!("impl key image vec check responding KeyImageSpentStatus")
    // if blockchain::key_image_spent(state, key_image).await? {
    //     Ok(KeyImageSpentStatus::SpentInBlockchain)
    // } else if todo!("key image is spent in tx pool") {
    //     Ok(KeyImageSpentStatus::SpentInPool)
    // } else {
    //     Ok(KeyImageSpentStatus::Unspent)
    // }
}

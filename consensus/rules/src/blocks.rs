use monero_serai::block::Block;
use primitive_types::U256;

use cryptonight_cuprate::*;

use crate::{
    current_unix_timestamp,
    hard_forks::HardForkError,
    miner_tx::{check_miner_tx, MinerTxError},
    HardFork,
};

const BLOCK_SIZE_SANITY_LEEWAY: usize = 100;
const BLOCK_FUTURE_TIME_LIMIT: u64 = 60 * 60 * 2;
const BLOCK_202612_POW_HASH: [u8; 32] =
    hex_literal::hex!("84f64766475d51837ac9efbef1926486e58563c95a19fef4aec3254f03000000");

const RX_SEEDHASH_EPOCH_BLOCKS: u64 = 2048;
const RX_SEEDHASH_EPOCH_LAG: u64 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BlockError {
    #[error("The blocks POW is invalid.")]
    POWInvalid,
    #[error("The block is too big.")]
    TooLarge,
    #[error("The block has too many transactions.")]
    TooManyTxs,
    #[error("The blocks previous ID is incorrect.")]
    PreviousIDIncorrect,
    #[error("The blocks timestamp is invalid.")]
    TimeStampInvalid,
    #[error("Hard-fork error: {0}")]
    HardForkError(#[from] HardForkError),
    #[error("Miner transaction error: {0}")]
    MinerTxError(#[from] MinerTxError),
}

pub trait RandomX {
    type Error;

    fn calculate_hash(&self, buf: &[u8]) -> Result<[u8; 32], Self::Error>;
}

pub fn is_randomx_seed_height(height: u64) -> bool {
    height % RX_SEEDHASH_EPOCH_BLOCKS == 0
}

pub fn randomx_seed_height(height: u64) -> u64 {
    if height <= RX_SEEDHASH_EPOCH_BLOCKS + RX_SEEDHASH_EPOCH_LAG {
        0
    } else {
        (height - RX_SEEDHASH_EPOCH_LAG - 1) & !(RX_SEEDHASH_EPOCH_BLOCKS - 1)
    }
}

/// Calculates the POW hash of this block.
pub fn calculate_pow_hash<R: RandomX>(
    randomx_vm: &R,
    buf: &[u8],
    height: u64,
    hf: &HardFork,
) -> Result<[u8; 32], BlockError> {
    if height == 202612 {
        return Ok(BLOCK_202612_POW_HASH);
    }

    Ok(if hf < &HardFork::V7 {
        cryptonight_hash_v0(buf)
    } else if hf == &HardFork::V7 {
        cryptonight_hash_v1(buf).map_err(|_| BlockError::POWInvalid)?
    } else if hf < &HardFork::V10 {
        cryptonight_hash_v2(buf)
    } else if hf < &HardFork::V12 {
        cryptonight_hash_r(buf, height)
    } else {
        randomx_vm
            .calculate_hash(buf)
            .map_err(|_| BlockError::POWInvalid)?
    })
}

/// Returns if the blocks POW hash is valid for the current difficulty.
///
/// See: https://cuprate.github.io/monero-book/consensus_rules/blocks/difficulty.html#checking-a-blocks-proof-of-work
pub fn check_block_pow(hash: &[u8; 32], difficulty: u128) -> Result<(), BlockError> {
    let int_hash = U256::from_little_endian(hash);

    let difficulty = U256::from(difficulty);

    if int_hash.checked_mul(difficulty).is_none() {
        tracing::debug!(
            "Invalid POW: {}, difficulty: {}",
            hex::encode(hash),
            difficulty
        );
        Err(BlockError::POWInvalid)
    } else {
        Ok(())
    }
}

/// Sanity check on the block blob size.
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks.html#block-weight-and-size
fn block_size_sanity_check(
    block_blob_len: usize,
    effective_median: usize,
) -> Result<(), BlockError> {
    if block_blob_len > effective_median * 2 + BLOCK_SIZE_SANITY_LEEWAY {
        Err(BlockError::TooLarge)
    } else {
        Ok(())
    }
}

/// Sanity check on number of txs in the block.
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks.html#amount-of-transactions
fn check_amount_txs(number_none_miner_txs: usize) -> Result<(), BlockError> {
    if number_none_miner_txs + 1 > 0x10000000 {
        Err(BlockError::TooManyTxs)
    } else {
        Ok(())
    }
}

/// Sanity check on the block weight.
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks.html#block-weight-and-siz
fn check_block_weight(
    block_weight: usize,
    median_for_block_reward: usize,
) -> Result<(), BlockError> {
    if block_weight > median_for_block_reward * 2 {
        Err(BlockError::TooLarge)
    } else {
        Ok(())
    }
}

/// Verifies the previous id is the last blocks hash
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks.html#previous-id
fn check_prev_id(block: &Block, top_hash: &[u8; 32]) -> Result<(), BlockError> {
    if &block.header.previous != top_hash {
        Err(BlockError::PreviousIDIncorrect)
    } else {
        Ok(())
    }
}

/// Checks the blocks timestamp is in the valid range.
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks.html#timestamp
fn check_timestamp(block: &Block, median_timestamp: u64) -> Result<(), BlockError> {
    if block.header.timestamp < median_timestamp
        || block.header.timestamp > current_unix_timestamp() + BLOCK_FUTURE_TIME_LIMIT
    {
        Err(BlockError::TimeStampInvalid)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ContextToVerifyBlock {
    pub median_weight_for_block_reward: usize,
    pub effective_median_weight: usize,
    pub top_hash: [u8; 32],
    pub median_block_timestamp: Option<u64>,
    pub chain_height: u64,
    pub current_hf: HardFork,
    pub next_difficulty: u128,
    pub already_generated_coins: u64,
}

/// Checks the block is valid returning the blocks hard-fork vote and the amount of coins generated.
///
/// Does not check the proof of work as that check is expensive and should be done last.
pub fn check_block(
    block: &Block,
    total_fees: u64,
    block_weight: usize,
    block_blob_len: usize,
    block_chain_ctx: &ContextToVerifyBlock,
) -> Result<(HardFork, u64), BlockError> {
    let (version, vote) = HardFork::from_block_header(&block.header)?;

    block_chain_ctx
        .current_hf
        .check_block_version_vote(&version, &vote)?;

    if let Some(median_timestamp) = block_chain_ctx.median_block_timestamp {
        check_timestamp(block, median_timestamp)?;
    }

    check_prev_id(block, &block_chain_ctx.top_hash)?;

    check_block_weight(block_weight, block_chain_ctx.median_weight_for_block_reward)?;
    block_size_sanity_check(block_blob_len, block_chain_ctx.effective_median_weight)?;

    check_amount_txs(block.txs.len())?;

    let generated_coins = check_miner_tx(
        &block.miner_tx,
        total_fees,
        block_chain_ctx.chain_height,
        block_weight,
        block_chain_ctx.median_weight_for_block_reward,
        block_chain_ctx.already_generated_coins,
        &block_chain_ctx.current_hf,
    )?;

    Ok((vote, generated_coins))
}

use std::collections::HashSet;

use crypto_bigint::{CheckedMul, U256};
use monero_oxide::block::Block;

use cuprate_cryptonight::*;

use crate::{
    check_block_version_vote, current_unix_timestamp,
    hard_forks::HardForkError,
    miner_tx::{check_miner_tx, MinerTxError},
    HardFork,
};

const BLOCK_SIZE_SANITY_LEEWAY: usize = 100;
const BLOCK_FUTURE_TIME_LIMIT: u64 = 60 * 60 * 2;
const BLOCK_202612_POW_HASH: [u8; 32] =
    hex_literal::hex!("84f64766475d51837ac9efbef1926486e58563c95a19fef4aec3254f03000000");

pub const PENALTY_FREE_ZONE_1: usize = 20000;
pub const PENALTY_FREE_ZONE_2: usize = 60000;
pub const PENALTY_FREE_ZONE_5: usize = 300000;

pub const RX_SEEDHASH_EPOCH_BLOCKS: usize = 2048;
pub const RX_SEEDHASH_EPOCH_LAG: usize = 64;

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
    #[error("The block contains a duplicate transaction.")]
    DuplicateTransaction,
    #[error("Hard-fork error: {0}")]
    HardForkError(#[from] HardForkError),
    #[error("Miner transaction error: {0}")]
    MinerTxError(#[from] MinerTxError),
}

/// A trait to represent the RandomX VM.
pub trait RandomX {
    type Error;

    fn calculate_hash(&self, buf: &[u8]) -> Result<[u8; 32], Self::Error>;
}

/// Returns if this height is a RandomX seed height.
pub const fn is_randomx_seed_height(height: usize) -> bool {
    height.is_multiple_of(RX_SEEDHASH_EPOCH_BLOCKS)
}

/// Returns the RandomX seed height for this block.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks.html#randomx-seed>
pub const fn randomx_seed_height(height: usize) -> usize {
    if height <= RX_SEEDHASH_EPOCH_BLOCKS + RX_SEEDHASH_EPOCH_LAG {
        0
    } else {
        (height - RX_SEEDHASH_EPOCH_LAG - 1) & !(RX_SEEDHASH_EPOCH_BLOCKS - 1)
    }
}

/// Calculates the POW hash of this block.
///
/// `randomx_vm` must be [`Some`] after hf 12.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks.html#pow-function>
pub fn calculate_pow_hash<R: RandomX>(
    randomx_vm: Option<&R>,
    buf: &[u8],
    height: usize,
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
        // FIXME: https://github.com/Cuprate/cuprate/issues/167.
        cryptonight_hash_r(buf, height as u64)
    } else {
        randomx_vm
            .expect("RandomX VM needed from hf 12")
            .calculate_hash(buf)
            .map_err(|_| BlockError::POWInvalid)?
    })
}

/// Returns if the blocks POW hash is valid for the current difficulty.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks.html#checking-pow-hash>
pub fn check_block_pow(hash: &[u8; 32], difficulty: u128) -> Result<(), BlockError> {
    let int_hash = U256::from_le_slice(hash);

    let difficulty = U256::from(difficulty);

    if int_hash.checked_mul(&difficulty).is_none().unwrap_u8() == 1 {
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

/// Returns the penalty free zone
///
/// <https://cuprate.github.io/monero-book/consensus_rules/blocks/weight_limit.html#penalty-free-zone>
pub fn penalty_free_zone(hf: HardFork) -> usize {
    if hf == HardFork::V1 {
        PENALTY_FREE_ZONE_1
    } else if hf >= HardFork::V2 && hf < HardFork::V5 {
        PENALTY_FREE_ZONE_2
    } else {
        PENALTY_FREE_ZONE_5
    }
}

/// Sanity check on the block blob size.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks.html#block-weight-and-size>
const fn block_size_sanity_check(
    block_blob_len: usize,
    effective_median: usize,
) -> Result<(), BlockError> {
    if block_blob_len > effective_median * 2 + BLOCK_SIZE_SANITY_LEEWAY {
        Err(BlockError::TooLarge)
    } else {
        Ok(())
    }
}

/// Sanity check on the block weight.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks.html#block-weight-and-size>
pub const fn check_block_weight(
    block_weight: usize,
    median_for_block_reward: usize,
) -> Result<(), BlockError> {
    if block_weight > median_for_block_reward * 2 {
        Err(BlockError::TooLarge)
    } else {
        Ok(())
    }
}

/// Sanity check on number of txs in the block.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks.html#amount-of-transactions>
const fn check_amount_txs(number_none_miner_txs: usize) -> Result<(), BlockError> {
    if number_none_miner_txs + 1 > 0x10000000 {
        Err(BlockError::TooManyTxs)
    } else {
        Ok(())
    }
}

/// Verifies the previous id is the last blocks hash
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks.html#previous-id>
fn check_prev_id(block: &Block, top_hash: &[u8; 32]) -> Result<(), BlockError> {
    if &block.header.previous == top_hash {
        Ok(())
    } else {
        Err(BlockError::PreviousIDIncorrect)
    }
}

/// Checks the blocks timestamp is in the valid range.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks.html#timestamp>
pub fn check_timestamp(block: &Block, median_timestamp: u64) -> Result<(), BlockError> {
    if block.header.timestamp < median_timestamp
        || block.header.timestamp > current_unix_timestamp() + BLOCK_FUTURE_TIME_LIMIT
    {
        Err(BlockError::TimeStampInvalid)
    } else {
        Ok(())
    }
}

/// Checks that all txs in the block have a unique hash.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks.html#no-duplicate-transactions>
fn check_txs_unique(txs: &[[u8; 32]]) -> Result<(), BlockError> {
    let set = txs.iter().collect::<HashSet<_>>();

    if set.len() == txs.len() {
        Ok(())
    } else {
        Err(BlockError::DuplicateTransaction)
    }
}

/// This struct contains the data needed to verify a block, implementers MUST make sure
/// the data in this struct is calculated correctly.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ContextToVerifyBlock {
    /// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/weights.html#median-weight-for-coinbase-checks>
    pub median_weight_for_block_reward: usize,
    /// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/weights.html#effective-median-weight>
    pub effective_median_weight: usize,
    /// The top hash of the blockchain, aka the block hash of the previous block to the one we are verifying.
    pub top_hash: [u8; 32],
    /// Contains the median timestamp over the last 60 blocks, if there is less than 60 blocks this should be [`None`]
    pub median_block_timestamp: Option<u64>,
    /// The current chain height.
    pub chain_height: usize,
    /// The current hard-fork.
    pub current_hf: HardFork,
    /// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/difficulty.html#calculating-difficulty>
    pub next_difficulty: u128,
    /// The amount of coins already minted.
    pub already_generated_coins: u64,
}

/// Checks the block is valid returning the blocks hard-fork `VOTE` and the amount of coins generated in this block.
///
/// This does not check the POW nor does it calculate the POW hash, this is because checking POW is very expensive and
/// to allow the computation of the POW hashes to be done separately. This also does not check the transactions in the
/// block are valid.
///
/// Missed block checks in this function:
///
/// <https://monero-book.cuprate.org/consensus_rules/blocks.html#key-images>
/// <https://monero-book.cuprate.org/consensus_rules/blocks.html#checking-pow-hash>
///
///
pub fn check_block(
    block: &Block,
    total_fees: u64,
    block_weight: usize,
    block_blob_len: usize,
    block_chain_ctx: &ContextToVerifyBlock,
) -> Result<(HardFork, u64), BlockError> {
    let (version, vote) =
        HardFork::from_block_header(&block.header).map_err(|_| HardForkError::HardForkUnknown)?;

    check_block_version_vote(&block_chain_ctx.current_hf, &version, &vote)?;

    if let Some(median_timestamp) = block_chain_ctx.median_block_timestamp {
        check_timestamp(block, median_timestamp)?;
    }

    check_prev_id(block, &block_chain_ctx.top_hash)?;

    check_block_weight(block_weight, block_chain_ctx.median_weight_for_block_reward)?;
    block_size_sanity_check(block_blob_len, block_chain_ctx.effective_median_weight)?;

    check_amount_txs(block.transactions.len())?;
    check_txs_unique(&block.transactions)?;

    let generated_coins = check_miner_tx(
        block.miner_transaction(),
        total_fees,
        block_chain_ctx.chain_height,
        block_weight,
        block_chain_ctx.median_weight_for_block_reward,
        block_chain_ctx.already_generated_coins,
        block_chain_ctx.current_hf,
    )?;

    Ok((vote, generated_coins))
}

#[cfg(test)]
mod tests {
    use proptest::{collection::vec, prelude::*};

    use super::*;

    proptest! {
        #[test]
        fn test_check_unique_txs(
            mut txs in vec(any::<[u8; 32]>(), 2..3000),
            duplicate in any::<[u8; 32]>(),
            dup_idx_1 in any::<usize>(),
            dup_idx_2 in any::<usize>(),
        ) {

            prop_assert!(check_txs_unique(&txs).is_ok());

            txs.insert(dup_idx_1 % txs.len(), duplicate);
            txs.insert(dup_idx_2 % txs.len(), duplicate);

            prop_assert!(check_txs_unique(&txs).is_err());
        }
    }
}

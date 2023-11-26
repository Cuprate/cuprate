use crypto_bigint::{CheckedMul, U256};
use monero_serai::block::Block;

use crate::{helper::current_time, ConsensusError};

const BLOCK_SIZE_SANITY_LEEWAY: usize = 100;
const BLOCK_FUTURE_TIME_LIMIT: u64 = 60 * 60 * 2;

/// Returns if the blocks POW hash is valid for the current difficulty.
///
/// See: https://cuprate.github.io/monero-book/consensus_rules/blocks/difficulty.html#checking-a-blocks-proof-of-work
pub fn check_block_pow(hash: &[u8; 32], difficulty: u128) -> Result<(), ConsensusError> {
    let int_hash = U256::from_le_slice(hash);

    let difficulty = U256::from_u128(difficulty);

    if int_hash.checked_mul(&difficulty).is_some().unwrap_u8() != 1 {
        Err(ConsensusError::BlockPOWInvalid)
    } else {
        Ok(())
    }
}

/// Sanity check on the block blob size.
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks.html#block-weight-and-size
pub fn block_size_sanity_check(
    block_blob_len: usize,
    effective_median: usize,
) -> Result<(), ConsensusError> {
    if block_blob_len > effective_median * 2 + BLOCK_SIZE_SANITY_LEEWAY {
        Err(ConsensusError::BlockIsTooLarge)
    } else {
        Ok(())
    }
}

/// Sanity check on number of txs in the block.
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks.html#amount-of-transactions
pub fn check_amount_txs(number_none_miner_txs: usize) -> Result<(), ConsensusError> {
    if number_none_miner_txs + 1 > 0x10000000 {
        Err(ConsensusError::BlockIsTooLarge)
    } else {
        Ok(())
    }
}

/// Sanity check on the block weight.
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks.html#block-weight-and-siz
pub fn block_weight_check(
    block_weight: usize,
    median_for_block_reward: usize,
) -> Result<(), ConsensusError> {
    if block_weight > median_for_block_reward * 2 {
        Err(ConsensusError::BlockIsTooLarge)
    } else {
        Ok(())
    }
}

/// Verifies the previous id is the last blocks hash
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks.html#previous-id
pub fn check_prev_id(block: &Block, top_hash: &[u8; 32]) -> Result<(), ConsensusError> {
    if &block.header.previous != top_hash {
        Err(ConsensusError::BlockIsNotApartOfChain)
    } else {
        Ok(())
    }
}

/// Checks the blocks timestamp is in the valid range.
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks.html#timestamp
pub fn check_timestamp(block: &Block, median_timestamp: u64) -> Result<(), ConsensusError> {
    if block.header.timestamp < median_timestamp
        || block.header.timestamp > current_time() + BLOCK_FUTURE_TIME_LIMIT
    {
        Err(ConsensusError::BlockTimestampInvalid)
    } else {
        Ok(())
    }
}

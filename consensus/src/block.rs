use monero_serai::block::Block;

use crate::{hardforks::BlockHFInfo, helper::current_time, ConsensusError};

pub mod difficulty;
pub mod pow;
pub mod reward;
pub mod weight;

pub use difficulty::{DifficultyCache, DifficultyCacheConfig};
pub use pow::{check_block_pow, BlockPOWInfo};
pub use weight::{block_weight, BlockWeightInfo, BlockWeightsCache, BlockWeightsCacheConfig};

const BLOCK_SIZE_SANITY_LEEWAY: usize = 100;
const BLOCK_FUTURE_TIME_LIMIT: u64 = 60 * 60 * 2;

pub struct BlockVerificationData {
    hf: BlockHFInfo,
    pow: BlockPOWInfo,
    weights: BlockWeightInfo,
    block_blob: Vec<u8>,
    block: Block,
    block_hash: [u8; 32],
    pow_hash: [u8; 32],
}

/// Sanity check on the block blob size.
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks.html#block-weight-and-size
fn block_size_sanity_check(
    block_blob_len: usize,
    effective_median: usize,
) -> Result<(), ConsensusError> {
    if block_blob_len > effective_median * 2 + BLOCK_SIZE_SANITY_LEEWAY {
        Err(ConsensusError::BlockIsTooLarge)
    } else {
        Ok(())
    }
}

/// Sanity check on the block weight.
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks.html#block-weight-and-siz
fn block_weight_check(
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
fn check_prev_id(block: &Block, top_hash: &[u8; 32]) -> Result<(), ConsensusError> {
    if &block.header.previous != top_hash {
        Err(ConsensusError::BlockIsNotApartOfChain)
    } else {
        Ok(())
    }
}

/// Checks the blocks timestamp is in the valid range.
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks.html#timestamp
fn check_timestamp(block: &Block, median_timestamp: u64) -> Result<(), ConsensusError> {
    if block.header.timestamp < median_timestamp
        || block.header.timestamp > current_time() + BLOCK_FUTURE_TIME_LIMIT
    {
        return Err(ConsensusError::BlockTimestampInvalid);
    } else {
        Ok(())
    }
}

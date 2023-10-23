use std::cmp::min;

use monero_serai::transaction::Timelock;

use crate::{context::difficulty::DifficultyCache, helper::current_time, HardFork};

const BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW: u64 = 60;

/// Checks if an outputs unlock time has passed.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/unlock_time.html#unlock-time
pub fn output_unlocked(
    time_lock: &Timelock,
    difficulty_cache: &DifficultyCache,
    current_chain_height: u64,
    hf: &HardFork,
) -> bool {
    match *time_lock {
        Timelock::None => true,
        Timelock::Block(unlock_height) => {
            check_block_time_lock(unlock_height.try_into().unwrap(), current_chain_height)
        }
        Timelock::Time(unlock_time) => {
            check_timestamp_time_lock(unlock_time, difficulty_cache, current_chain_height, hf)
        }
    }
}

/// Returns if a locked output, which uses a block height, can be spend.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/unlock_time.html#block-height
fn check_block_time_lock(unlock_height: u64, current_chain_height: u64) -> bool {
    // current_chain_height = 1 + top height
    unlock_height >= current_chain_height
}

/// Returns the timestamp the should be used when checking locked outputs.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/unlock_time.html#getting-the-current-time
fn get_current_timestamp(
    difficulty_cache: &DifficultyCache,
    current_chain_height: u64,
    hf: &HardFork,
) -> u64 {
    if hf < &HardFork::V13 || current_chain_height < BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW {
        current_time()
    } else {
        let median = difficulty_cache
            .median_timestamp(BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW.try_into().unwrap());
        let adjusted_median =
            median + (BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW + 1) * hf.block_time().as_secs() / 2;

        // This is safe as we just check we don't have less than 60 blocks in the chain.
        let adjusted_top_block =
            difficulty_cache.top_block_timestamp().unwrap() + hf.block_time().as_secs();

        min(adjusted_median, adjusted_top_block)
    }
}

/// Returns if a locked output, which uses a block height, can be spend.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/unlock_time.html#timestamp
fn check_timestamp_time_lock(
    unlock_timestamp: u64,
    difficulty_cache: &DifficultyCache,
    current_chain_height: u64,
    hf: &HardFork,
) -> bool {
    let timestamp = get_current_timestamp(difficulty_cache, current_chain_height, hf);
    timestamp + hf.block_time().as_secs() >= unlock_timestamp
}

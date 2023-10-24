//! # Time Locks
//!
//! This module contains the checks for time locks, using the `check_all_time_locks` function.
//!
use monero_serai::transaction::Timelock;

use crate::{ConsensusError, HardFork};

/// Checks all the time locks are unlocked.
///
/// `current_time_lock_timestamp` must be: https://cuprate.github.io/monero-book/consensus_rules/transactions/unlock_time.html#getting-the-current-time
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/unlock_time.html#unlock-time
pub fn check_all_time_locks(
    time_locks: &[Timelock],
    current_chain_height: u64,
    current_time_lock_timestamp: u64,
    hf: &HardFork,
) -> Result<(), ConsensusError> {
    time_locks.iter().try_for_each(|time_lock| {
        if !output_unlocked(
            time_lock,
            current_chain_height,
            current_time_lock_timestamp,
            hf,
        ) {
            Err(ConsensusError::TransactionHasInvalidRing(
                "One or more ring members locked",
            ))
        } else {
            Ok(())
        }
    })
}

/// Checks if an outputs unlock time has passed.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/unlock_time.html#unlock-time
fn output_unlocked(
    time_lock: &Timelock,
    current_chain_height: u64,
    current_time_lock_timestamp: u64,
    hf: &HardFork,
) -> bool {
    match *time_lock {
        Timelock::None => true,
        Timelock::Block(unlock_height) => {
            check_block_time_lock(unlock_height.try_into().unwrap(), current_chain_height)
        }
        Timelock::Time(unlock_time) => {
            check_timestamp_time_lock(unlock_time, current_time_lock_timestamp, hf)
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

/// ///
/// Returns if a locked output, which uses a block height, can be spend.
///
/// https://cuprate.github.io/monero-book/consensus_rules/transactions/unlock_time.html#timestamp
fn check_timestamp_time_lock(
    unlock_timestamp: u64,
    current_time_lock_timestamp: u64,
    hf: &HardFork,
) -> bool {
    current_time_lock_timestamp + hf.block_time().as_secs() >= unlock_timestamp
}

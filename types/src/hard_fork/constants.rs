//! TODO

//---------------------------------------------------------------------------------------------------- Import
use std::time::Duration;

//---------------------------------------------------------------------------------------------------- Import
/// Target block time for hf 1.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/difficulty.html#target-seconds>
pub const BLOCK_TIME_V1: Duration = Duration::from_secs(60);

/// Target block time from v2.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/difficulty.html#target-seconds>
pub const BLOCK_TIME_V2: Duration = Duration::from_secs(120);

/// TODO
pub const NUMB_OF_HARD_FORKS: usize = 16;

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

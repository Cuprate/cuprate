//! TODO

//---------------------------------------------------------------------------------------------------- Import
use bytemuck::{AnyBitPattern, NoUninit, Pod, Zeroable};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::hard_fork::HardFork;

//---------------------------------------------------------------------------------------------------- ExtendedBlockHeader
/// TODO
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExtendedBlockHeader {
    /// TODO
    pub version: HardFork,
    /// TODO
    pub vote: HardFork,
    /// TODO
    pub timestamp: u64,
    /// TODO
    pub cumulative_difficulty: u128,
    /// TODO
    pub block_weight: usize,
    /// TODO
    pub long_term_weight: usize,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

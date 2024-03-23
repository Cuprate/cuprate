//! TODO

//---------------------------------------------------------------------------------------------------- Import
use std::{
    collections::VecDeque,
    fmt::{Display, Formatter},
    time::Duration,
};

use bytemuck::{Pod, Zeroable};
use monero_serai::block::BlockHeader;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::hard_fork::{constants::NUMB_OF_HARD_FORKS, error::HardForkError, hard_fork::HardFork};

//---------------------------------------------------------------------------------------------------- HFInfo
/// Information about a given hard-fork.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HFInfo {
    /// TODO
    pub(super) height: u64,
    /// TODO
    pub(super) threshold: u64,
}

impl HFInfo {
    /// TODO
    pub const fn new(height: u64, threshold: u64) -> Self {
        Self { height, threshold }
    }
}

//---------------------------------------------------------------------------------------------------- HFsInfo
/// Information about every hard-fork Monero has had.
#[derive(Debug, Clone, Copy)]
pub struct HFsInfo([HFInfo; NUMB_OF_HARD_FORKS]);

impl HFsInfo {
    /// TODO
    pub const fn new(hfs: [HFInfo; NUMB_OF_HARD_FORKS]) -> Self {
        Self(hfs)
    }

    /// TODO
    pub const fn info_for_hf(&self, hf: &HardFork) -> HFInfo {
        self.0[*hf as usize - 1]
    }

    /// Returns the main-net hard-fork information.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#Mainnet-Hard-Forks>
    pub const fn main_net() -> Self {
        Self([
            HFInfo::new(0, 0),
            HFInfo::new(1_009_827, 0),
            HFInfo::new(1_141_317, 0),
            HFInfo::new(1_220_516, 0),
            HFInfo::new(1_288_616, 0),
            HFInfo::new(1_400_000, 0),
            HFInfo::new(1_546_000, 0),
            HFInfo::new(1_685_555, 0),
            HFInfo::new(1_686_275, 0),
            HFInfo::new(1_788_000, 0),
            HFInfo::new(1_788_720, 0),
            HFInfo::new(1_978_433, 0),
            HFInfo::new(2_210_000, 0),
            HFInfo::new(2_210_720, 0),
            HFInfo::new(2_688_888, 0),
            HFInfo::new(2_689_608, 0),
        ])
    }

    /// Returns the test-net hard-fork information.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#Testnet-Hard-Forks>
    pub const fn test_net() -> Self {
        Self([
            HFInfo::new(0, 0),
            HFInfo::new(624_634, 0),
            HFInfo::new(800_500, 0),
            HFInfo::new(801_219, 0),
            HFInfo::new(802_660, 0),
            HFInfo::new(971_400, 0),
            HFInfo::new(1_057_027, 0),
            HFInfo::new(1_057_058, 0),
            HFInfo::new(1_057_778, 0),
            HFInfo::new(1_154_318, 0),
            HFInfo::new(1_155_038, 0),
            HFInfo::new(1_308_737, 0),
            HFInfo::new(1_543_939, 0),
            HFInfo::new(1_544_659, 0),
            HFInfo::new(1_982_800, 0),
            HFInfo::new(1_983_520, 0),
        ])
    }

    /// Returns the test-net hard-fork information.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#Stagenet-Hard-Forks>
    pub const fn stage_net() -> Self {
        Self([
            HFInfo::new(0, 0),
            HFInfo::new(32000, 0),
            HFInfo::new(33000, 0),
            HFInfo::new(34000, 0),
            HFInfo::new(35000, 0),
            HFInfo::new(36000, 0),
            HFInfo::new(37000, 0),
            HFInfo::new(176_456, 0),
            HFInfo::new(177_176, 0),
            HFInfo::new(269_000, 0),
            HFInfo::new(269_720, 0),
            HFInfo::new(454_721, 0),
            HFInfo::new(675_405, 0),
            HFInfo::new(676_125, 0),
            HFInfo::new(1_151_000, 0),
            HFInfo::new(1_151_720, 0),
        ])
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

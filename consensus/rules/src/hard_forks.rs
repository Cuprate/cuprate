//! # Hard-Forks
//!
//! Monero use hard-forks to update it's protocol, this module contains a [`HardFork`] enum which is
//! an identifier for every current hard-fork.
//!
//! This module also contains a [`HFVotes`] struct which keeps track of current blockchain voting, and
//! has a method [`HFVotes::current_fork`] to check if the next hard-fork should be activated.
//!
use monero_serai::block::BlockHeader;
use std::{
    collections::VecDeque,
    fmt::{Display, Formatter},
    time::Duration,
};

#[cfg(test)]
mod tests;

/// Target block time for hf 1.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/difficulty.html#target-seconds>
const BLOCK_TIME_V1: Duration = Duration::from_secs(60);
/// Target block time from v2.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/difficulty.html#target-seconds>
const BLOCK_TIME_V2: Duration = Duration::from_secs(120);

pub const NUMB_OF_HARD_FORKS: usize = 16;

#[derive(Debug, Copy, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HardForkError {
    #[error("The hard-fork is unknown")]
    HardForkUnknown,
    #[error("The block is on an incorrect hard-fork")]
    VersionIncorrect,
    #[error("The block's vote is for a previous hard-fork")]
    VoteTooLow,
}

/// Information about a given hard-fork.
#[derive(Debug, Clone, Copy)]
pub struct HFInfo {
    height: u64,
    threshold: u64,
}
impl HFInfo {
    pub const fn new(height: u64, threshold: u64) -> HFInfo {
        HFInfo { height, threshold }
    }
}

/// Information about every hard-fork Monero has had.
#[derive(Debug, Clone, Copy)]
pub struct HFsInfo([HFInfo; NUMB_OF_HARD_FORKS]);

impl HFsInfo {
    pub fn info_for_hf(&self, hf: &HardFork) -> HFInfo {
        self.0[*hf as usize - 1]
    }

    pub const fn new(hfs: [HFInfo; NUMB_OF_HARD_FORKS]) -> Self {
        Self(hfs)
    }

    /// Returns the main-net hard-fork information.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#Mainnet-Hard-Forks>
    pub const fn main_net() -> HFsInfo {
        Self([
            HFInfo::new(0, 0),
            HFInfo::new(1009827, 0),
            HFInfo::new(1141317, 0),
            HFInfo::new(1220516, 0),
            HFInfo::new(1288616, 0),
            HFInfo::new(1400000, 0),
            HFInfo::new(1546000, 0),
            HFInfo::new(1685555, 0),
            HFInfo::new(1686275, 0),
            HFInfo::new(1788000, 0),
            HFInfo::new(1788720, 0),
            HFInfo::new(1978433, 0),
            HFInfo::new(2210000, 0),
            HFInfo::new(2210720, 0),
            HFInfo::new(2688888, 0),
            HFInfo::new(2689608, 0),
        ])
    }

    /// Returns the test-net hard-fork information.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#Testnet-Hard-Forks>
    pub const fn test_net() -> HFsInfo {
        Self([
            HFInfo::new(0, 0),
            HFInfo::new(624634, 0),
            HFInfo::new(800500, 0),
            HFInfo::new(801219, 0),
            HFInfo::new(802660, 0),
            HFInfo::new(971400, 0),
            HFInfo::new(1057027, 0),
            HFInfo::new(1057058, 0),
            HFInfo::new(1057778, 0),
            HFInfo::new(1154318, 0),
            HFInfo::new(1155038, 0),
            HFInfo::new(1308737, 0),
            HFInfo::new(1543939, 0),
            HFInfo::new(1544659, 0),
            HFInfo::new(1982800, 0),
            HFInfo::new(1983520, 0),
        ])
    }

    /// Returns the test-net hard-fork information.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#Stagenet-Hard-Forks>
    pub const fn stage_net() -> HFsInfo {
        Self([
            HFInfo::new(0, 0),
            HFInfo::new(32000, 0),
            HFInfo::new(33000, 0),
            HFInfo::new(34000, 0),
            HFInfo::new(35000, 0),
            HFInfo::new(36000, 0),
            HFInfo::new(37000, 0),
            HFInfo::new(176456, 0),
            HFInfo::new(177176, 0),
            HFInfo::new(269000, 0),
            HFInfo::new(269720, 0),
            HFInfo::new(454721, 0),
            HFInfo::new(675405, 0),
            HFInfo::new(676125, 0),
            HFInfo::new(1151000, 0),
            HFInfo::new(1151720, 0),
        ])
    }
}

/// An identifier for every hard-fork Monero has had.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
#[cfg_attr(any(feature = "proptest", test), derive(proptest_derive::Arbitrary))]
#[repr(u8)]
pub enum HardFork {
    V1 = 1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    V8,
    V9,
    V10,
    V11,
    V12,
    V13,
    V14,
    V15,
    // remember to update from_vote!
    V16,
}

impl HardFork {
    /// Returns the hard-fork for a blocks `major_version` field.
    ///
    /// <https://monero-book.cuprate.org/consensus_rules/hardforks.html#blocks-version-and-vote>
    #[inline]
    pub fn from_version(version: u8) -> Result<HardFork, HardForkError> {
        Ok(match version {
            1 => HardFork::V1,
            2 => HardFork::V2,
            3 => HardFork::V3,
            4 => HardFork::V4,
            5 => HardFork::V5,
            6 => HardFork::V6,
            7 => HardFork::V7,
            8 => HardFork::V8,
            9 => HardFork::V9,
            10 => HardFork::V10,
            11 => HardFork::V11,
            12 => HardFork::V12,
            13 => HardFork::V13,
            14 => HardFork::V14,
            15 => HardFork::V15,
            16 => HardFork::V16,
            _ => return Err(HardForkError::HardForkUnknown),
        })
    }

    /// Returns the hard-fork for a blocks `minor_version` (vote) field.
    ///
    /// <https://monero-book.cuprate.org/consensus_rules/hardforks.html#blocks-version-and-vote>
    #[inline]
    pub fn from_vote(vote: u8) -> HardFork {
        if vote == 0 {
            // A vote of 0 is interpreted as 1 as that's what Monero used to default to.
            return HardFork::V1;
        }
        // This must default to the latest hard-fork!
        Self::from_version(vote).unwrap_or(HardFork::V16)
    }

    #[inline]
    pub fn from_block_header(header: &BlockHeader) -> Result<(HardFork, HardFork), HardForkError> {
        Ok((
            HardFork::from_version(header.major_version)?,
            HardFork::from_vote(header.minor_version),
        ))
    }

    /// Returns the next hard-fork.
    pub fn next_fork(&self) -> Option<HardFork> {
        HardFork::from_version(*self as u8 + 1).ok()
    }

    /// Returns the target block time for this hardfork.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/blocks/difficulty.html#target-seconds>
    pub fn block_time(&self) -> Duration {
        match self {
            HardFork::V1 => BLOCK_TIME_V1,
            _ => BLOCK_TIME_V2,
        }
    }

    /// Checks a blocks version and vote, assuming that `self` is the current hard-fork.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#blocks-version-and-vote>
    pub fn check_block_version_vote(
        &self,
        version: &HardFork,
        vote: &HardFork,
    ) -> Result<(), HardForkError> {
        // self = current hf
        if self != version {
            Err(HardForkError::VersionIncorrect)?;
        }
        if self > vote {
            Err(HardForkError::VoteTooLow)?;
        }

        Ok(())
    }
}

/// A struct holding the current voting state of the blockchain.
#[derive(Debug, Clone)]
pub struct HFVotes {
    votes: [u64; NUMB_OF_HARD_FORKS],
    vote_list: VecDeque<HardFork>,
    window_size: usize,
}

impl Display for HFVotes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HFVotes")
            .field("total", &self.total_votes())
            .field("V1", &self.votes_for_hf(&HardFork::V1))
            .field("V2", &self.votes_for_hf(&HardFork::V2))
            .field("V3", &self.votes_for_hf(&HardFork::V3))
            .field("V4", &self.votes_for_hf(&HardFork::V4))
            .field("V5", &self.votes_for_hf(&HardFork::V5))
            .field("V6", &self.votes_for_hf(&HardFork::V6))
            .field("V7", &self.votes_for_hf(&HardFork::V7))
            .field("V8", &self.votes_for_hf(&HardFork::V8))
            .field("V9", &self.votes_for_hf(&HardFork::V9))
            .field("V10", &self.votes_for_hf(&HardFork::V10))
            .field("V11", &self.votes_for_hf(&HardFork::V11))
            .field("V12", &self.votes_for_hf(&HardFork::V12))
            .field("V13", &self.votes_for_hf(&HardFork::V13))
            .field("V14", &self.votes_for_hf(&HardFork::V14))
            .field("V15", &self.votes_for_hf(&HardFork::V15))
            .field("V16", &self.votes_for_hf(&HardFork::V16))
            .finish()
    }
}

impl HFVotes {
    pub fn new(window_size: usize) -> HFVotes {
        HFVotes {
            votes: [0; NUMB_OF_HARD_FORKS],
            vote_list: VecDeque::with_capacity(window_size),
            window_size,
        }
    }

    /// Add a vote for a hard-fork, this function removes votes outside of the window.
    pub fn add_vote_for_hf(&mut self, hf: &HardFork) {
        self.vote_list.push_back(*hf);
        self.votes[*hf as usize - 1] += 1;
        if self.vote_list.len() > self.window_size {
            let hf = self.vote_list.pop_front().unwrap();
            self.votes[hf as usize - 1] -= 1;
        }
    }

    /// Returns the total votes for a hard-fork.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#accepting-a-fork>
    pub fn votes_for_hf(&self, hf: &HardFork) -> u64 {
        self.votes[*hf as usize - 1..].iter().sum()
    }

    /// Returns the total amount of votes being tracked
    pub fn total_votes(&self) -> u64 {
        self.votes.iter().sum()
    }

    /// Checks if a future hard fork should be activated, returning the next hard-fork that should be
    /// activated.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#accepting-a-fork>
    pub fn current_fork(
        &self,
        current_hf: &HardFork,
        current_height: u64,
        window: u64,
        hfs_info: &HFsInfo,
    ) -> HardFork {
        let mut current_hf = *current_hf;

        while let Some(next_hf) = current_hf.next_fork() {
            let hf_info = hfs_info.info_for_hf(&next_hf);
            if current_height >= hf_info.height
                && self.votes_for_hf(&next_hf) >= votes_needed(hf_info.threshold, window)
            {
                current_hf = next_hf;
            } else {
                // if we don't have enough votes for this fork any future fork won't have enough votes
                // as votes are cumulative.
                // TODO: If a future fork has a lower threshold that could not be true, but as all current forks
                // have threshold 0 it is ok for now.
                return current_hf;
            }
        }
        current_hf
    }
}

/// Returns the votes needed for a hard-fork.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#accepting-a-fork>
pub fn votes_needed(threshold: u64, window: u64) -> u64 {
    (threshold * window).div_ceil(100)
}

//! # Hard-Forks
//!
//! Monero use hard-forks to update it's protocol, this module contains a [`HFVotes`] struct which
//! keeps track of current blockchain voting, and has a method [`HFVotes::current_fork`] to check
//! if the next hard-fork should be activated.
use std::{
    collections::VecDeque,
    fmt::{Display, Formatter},
};

pub use cuprate_types::{HardFork, HardForkError};

#[cfg(test)]
mod tests;

pub const NUMB_OF_HARD_FORKS: usize = 16;

/// Checks a blocks version and vote, assuming that `hf` is the current hard-fork.
///
/// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#blocks-version-and-vote>
pub fn check_block_version_vote(
    hf: &HardFork,
    version: &HardFork,
    vote: &HardFork,
) -> Result<(), HardForkError> {
    // self = current hf
    if hf != version {
        return Err(HardForkError::VersionIncorrect);
    }
    if hf > vote {
        return Err(HardForkError::VoteTooLow);
    }

    Ok(())
}

/// Information about a given hard-fork.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct HFInfo {
    height: usize,
    threshold: usize,
}
impl HFInfo {
    pub const fn height(&self) -> usize {
        self.height
    }

    pub const fn threshold(&self) -> usize {
        self.threshold
    }

    pub const fn new(height: usize, threshold: usize) -> Self {
        Self { height, threshold }
    }
}

/// Information about every hard-fork Monero has had.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct HFsInfo([HFInfo; NUMB_OF_HARD_FORKS]);

impl HFsInfo {
    pub const fn info_for_hf(&self, hf: &HardFork) -> HFInfo {
        self.0[*hf as usize - 1]
    }

    pub const fn new(hfs: [HFInfo; NUMB_OF_HARD_FORKS]) -> Self {
        Self(hfs)
    }

    /// Returns the main-net hard-fork information.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#Mainnet-Hard-Forks>
    pub const fn main_net() -> Self {
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
    pub const fn test_net() -> Self {
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
    pub const fn stage_net() -> Self {
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

/// A struct holding the current voting state of the blockchain.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HFVotes {
    votes: [usize; NUMB_OF_HARD_FORKS],
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
    pub fn new(window_size: usize) -> Self {
        Self {
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

    /// Pop a number of blocks from the top of the cache and push some values into the front of the cache,
    /// i.e. the oldest blocks.
    ///
    /// `old_block_votes` should contain the HFs below the window that now will be in the window after popping
    /// blocks from the top.
    ///
    /// # Panics
    ///
    /// This will panic if `old_block_votes` contains more HFs than `numb_blocks`.
    pub fn reverse_blocks(&mut self, numb_blocks: usize, old_block_votes: Self) {
        assert!(old_block_votes.vote_list.len() <= numb_blocks);

        for hf in self.vote_list.drain(self.vote_list.len() - numb_blocks..) {
            self.votes[hf as usize - 1] -= 1;
        }

        for old_vote in old_block_votes.vote_list.into_iter().rev() {
            self.vote_list.push_front(old_vote);
            self.votes[old_vote as usize - 1] += 1;
        }
    }

    /// Returns the total votes for a hard-fork.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#accepting-a-fork>
    pub fn votes_for_hf(&self, hf: &HardFork) -> usize {
        self.votes[*hf as usize - 1..].iter().sum()
    }

    /// Returns the total amount of votes being tracked
    pub fn total_votes(&self) -> usize {
        self.vote_list.len()
    }

    /// Checks if a future hard fork should be activated, returning the next hard-fork that should be
    /// activated.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#accepting-a-fork>
    pub fn current_fork(
        &self,
        current_hf: &HardFork,
        current_height: usize,
        window: usize,
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
pub const fn votes_needed(threshold: usize, window: usize) -> usize {
    (threshold * window).div_ceil(100)
}

//! TODO

//---------------------------------------------------------------------------------------------------- Import
use std::{
    collections::VecDeque,
    fmt::{Display, Formatter},
    time::Duration,
};

use bytemuck::{AnyBitPattern, NoUninit, Pod, Zeroable};
use monero_serai::block::BlockHeader;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::hard_fork::{
    constants::NUMB_OF_HARD_FORKS, error::HardForkError, hard_fork::HardFork, info::HFsInfo,
};

//---------------------------------------------------------------------------------------------------- HFVotes
/// A struct holding the current voting state of the blockchain.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HFVotes {
    /// TODO
    votes: [u64; NUMB_OF_HARD_FORKS],
    /// TODO
    vote_list: VecDeque<HardFork>,
    /// TODO
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
    /// TODO
    pub fn new(window_size: usize) -> Self {
        Self {
            votes: [0; NUMB_OF_HARD_FORKS],
            vote_list: VecDeque::with_capacity(window_size),
            window_size,
        }
    }

    /// Add a vote for a hard-fork, this function removes votes outside of the window.
    ///
    /// # Panics
    /// TODO
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
    #[allow(clippy::similar_names)]
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
pub const fn votes_needed(threshold: u64, window: u64) -> u64 {
    (threshold * window).div_ceil(100)
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

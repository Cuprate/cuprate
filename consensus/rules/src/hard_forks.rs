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

/// The amount of time in seconds after the last fork's scheduled timestamp before we
/// consider this node likely forked from the network.
///
/// ref: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_basic/hardfork.h#L49>
const FORKED_TIME: u64 = 31_557_600; // one year in seconds

/// The amount of time in seconds after the last fork's scheduled timestamp before we
/// warn that an update is needed.
///
/// ref: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_basic/hardfork.h#L50>
const UPDATE_TIME: u64 = FORKED_TIME / 2;

/// The state of the daemon with respect to the latest scheduled hard-fork.
///
/// ref: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_basic/hardfork.h#L46>
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(u32)]
pub enum HardForkState {
    LikelyForked = 0,
    UpdateNeeded = 1,
    Ready = 2,
}

impl From<HardForkState> for u32 {
    fn from(state: HardForkState) -> Self {
        state as Self
    }
}

/// Information about a given hard-fork.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct HFInfo {
    height: usize,
    threshold: usize,
    time: u64,
}
impl HFInfo {
    pub const fn height(&self) -> usize {
        self.height
    }

    pub const fn threshold(&self) -> usize {
        self.threshold
    }

    pub const fn time(&self) -> u64 {
        self.time
    }

    pub const fn new(height: usize, threshold: usize, time: u64) -> Self {
        Self {
            height,
            threshold,
            time,
        }
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

    /// Returns the hard-fork state based on the current time.
    ///
    /// Mirrors `HardFork::get_state()` in monerod:
    /// - [`HardForkState::LikelyForked`]: more than one year past the last scheduled fork time.
    /// - [`HardForkState::UpdateNeeded`]: more than six months past the last scheduled fork time.
    /// - [`HardForkState::Ready`]: within six months of the last scheduled fork time.
    ///
    /// ref: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_basic/hardfork.cpp#L326-L345>
    pub const fn hard_fork_state(&self, current_time: u64) -> HardForkState {
        let last_fork_time = self.0[NUMB_OF_HARD_FORKS - 1].time;
        if current_time >= last_fork_time.saturating_add(FORKED_TIME) {
            HardForkState::LikelyForked
        } else if current_time >= last_fork_time.saturating_add(UPDATE_TIME) {
            HardForkState::UpdateNeeded
        } else {
            HardForkState::Ready
        }
    }

    /// Returns the main-net hard-fork information.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#Mainnet-Hard-Forks>
    pub const fn main_net() -> Self {
        Self([
            HFInfo::new(0, 0, 1341378000),
            HFInfo::new(1009827, 0, 1442763710),
            HFInfo::new(1141317, 0, 1458558528),
            HFInfo::new(1220516, 0, 1483574400),
            HFInfo::new(1288616, 0, 1489520158),
            HFInfo::new(1400000, 0, 1503046577),
            HFInfo::new(1546000, 0, 1521303150),
            HFInfo::new(1685555, 0, 1535889547),
            HFInfo::new(1686275, 0, 1535889548),
            HFInfo::new(1788000, 0, 1549792439),
            HFInfo::new(1788720, 0, 1550225678),
            HFInfo::new(1978433, 0, 1571419280),
            HFInfo::new(2210000, 0, 1598180817),
            HFInfo::new(2210720, 0, 1598180818),
            HFInfo::new(2688888, 0, 1656629117),
            HFInfo::new(2689608, 0, 1656629118),
        ])
    }

    /// Returns the test-net hard-fork information.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#Testnet-Hard-Forks>
    pub const fn test_net() -> Self {
        Self([
            HFInfo::new(0, 0, 1341378000),
            HFInfo::new(624634, 0, 1445355000),
            HFInfo::new(800500, 0, 1472415034),
            HFInfo::new(801219, 0, 1472415035),
            HFInfo::new(802660, 0, 1472415036 + 86400 * 180),
            HFInfo::new(971400, 0, 1501709789),
            HFInfo::new(1057027, 0, 1512211236),
            HFInfo::new(1057058, 0, 1533211200),
            HFInfo::new(1057778, 0, 1533297600),
            HFInfo::new(1154318, 0, 1550153694),
            HFInfo::new(1155038, 0, 1550225678),
            HFInfo::new(1308737, 0, 1569582000),
            HFInfo::new(1543939, 0, 1599069376),
            HFInfo::new(1544659, 0, 1599069377),
            HFInfo::new(1982800, 0, 1652727000),
            HFInfo::new(1983520, 0, 1652813400),
        ])
    }

    /// Returns the fake-chain (regtest) hard-fork information.
    ///
    /// ref: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_core/cryptonote_core.cpp#L670>
    pub const fn fake_chain() -> Self {
        let mut hfs = [HFInfo::new(1, 0, 0); NUMB_OF_HARD_FORKS];
        hfs[0] = HFInfo::new(0, 0, 0);
        Self(hfs)
    }

    /// Returns the stagenet hard-fork information.
    ///
    /// ref: <https://monero-book.cuprate.org/consensus_rules/hardforks.html#Stagenet-Hard-Forks>
    pub const fn stage_net() -> Self {
        Self([
            HFInfo::new(0, 0, 1341378000),
            HFInfo::new(32000, 0, 1521000000),
            HFInfo::new(33000, 0, 1521120000),
            HFInfo::new(34000, 0, 1521240000),
            HFInfo::new(35000, 0, 1521360000),
            HFInfo::new(36000, 0, 1521480000),
            HFInfo::new(37000, 0, 1521600000),
            HFInfo::new(176456, 0, 1537821770),
            HFInfo::new(177176, 0, 1537821771),
            HFInfo::new(269000, 0, 1550153694),
            HFInfo::new(269720, 0, 1550225678),
            HFInfo::new(454721, 0, 1571419280),
            HFInfo::new(675405, 0, 1598180817),
            HFInfo::new(676125, 0, 1598180818),
            HFInfo::new(1151000, 0, 1656629117),
            HFInfo::new(1151720, 0, 1656629118),
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

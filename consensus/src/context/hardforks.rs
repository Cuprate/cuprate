use std::{
    fmt::{Display, Formatter},
    ops::Range,
    time::Duration,
};

use monero_serai::block::BlockHeader;
use tower::ServiceExt;
use tracing::instrument;

use crate::{ConsensusError, Database, DatabaseRequest, DatabaseResponse};

// https://cuprate.github.io/monero-docs/consensus_rules/hardforks.html#accepting-a-fork
const DEFAULT_WINDOW_SIZE: u64 = 10080; // supermajority window check length - a week
const BLOCK_TIME_V1: Duration = Duration::from_secs(60);
const BLOCK_TIME_V2: Duration = Duration::from_secs(120);

const NUMB_OF_HARD_FORKS: usize = 16;

#[derive(Debug, Clone, Copy)]
pub struct BlockHFInfo {
    pub version: HardFork,
    pub vote: HardFork,
}

impl BlockHFInfo {
    pub fn from_block_header(block_header: &BlockHeader) -> Result<BlockHFInfo, ConsensusError> {
        BlockHFInfo::from_major_minor(block_header.major_version, block_header.minor_version)
    }

    pub fn from_major_minor(
        major_version: u8,
        minor_version: u8,
    ) -> Result<BlockHFInfo, ConsensusError> {
        Ok(BlockHFInfo {
            version: HardFork::from_version(&major_version)?,
            vote: HardFork::from_vote(&minor_version),
        })
    }
}

/// Information about a given hard-fork.
#[derive(Debug, Clone, Copy)]
pub struct HFInfo {
    height: u64,
    threshold: u64,
}
impl HFInfo {
    pub fn new(height: u64, threshold: u64) -> HFInfo {
        HFInfo { height, threshold }
    }

    /// Returns the main-net hard-fork information.
    ///
    /// https://cuprate.github.io/monero-book/consensus_rules/hardforks.html#Mainnet-Hard-Forks
    pub fn main_net() -> [HFInfo; NUMB_OF_HARD_FORKS] {
        [
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
        ]
    }
}

/// Configuration for hard-forks.
///
#[derive(Debug, Clone)]
pub struct HardForkConfig {
    /// The network we are on.
    forks: [HFInfo; NUMB_OF_HARD_FORKS],
    /// The amount of votes we are taking into account to decide on a fork activation.
    window: u64,
}

impl HardForkConfig {
    fn fork_info(&self, hf: &HardFork) -> HFInfo {
        self.forks[*hf as usize - 1]
    }

    pub fn main_net() -> HardForkConfig {
        Self {
            forks: HFInfo::main_net(),
            window: DEFAULT_WINDOW_SIZE,
        }
    }
}

/// An identifier for every hard-fork Monero has had.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
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
    /// https://cuprate.github.io/monero-docs/consensus_rules/hardforks.html#blocks-version-and-vote
    pub fn from_version(version: &u8) -> Result<HardFork, ConsensusError> {
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
            _ => {
                return Err(ConsensusError::InvalidHardForkVersion(
                    "Version is not a known hard fork",
                ))
            }
        })
    }

    /// Returns the hard-fork for a blocks `minor_version` (vote) field.
    ///
    /// https://cuprate.github.io/monero-docs/consensus_rules/hardforks.html#blocks-version-and-vote
    pub fn from_vote(vote: &u8) -> HardFork {
        if *vote == 0 {
            // A vote of 0 is interpreted as 1 as that's what Monero used to default to.
            return HardFork::V1;
        }
        // This must default to the latest hard-fork!
        Self::from_version(vote).unwrap_or(HardFork::V16)
    }

    /// Returns the next hard-fork.
    pub fn next_fork(&self) -> Option<HardFork> {
        HardFork::from_version(&(*self as u8 + 1)).ok()
    }

    /// Returns if the hard-fork is in range:
    ///
    /// start <= hf < end
    pub fn in_range(&self, start: &HardFork, end: &HardFork) -> bool {
        start <= self && self < end
    }

    /// Returns the target block time for this hardfork.
    pub fn block_time(&self) -> Duration {
        match self {
            HardFork::V1 => BLOCK_TIME_V1,
            _ => BLOCK_TIME_V2,
        }
    }

    /// Checks a blocks version and vote, assuming that `self` is the current hard-fork.
    ///
    /// https://cuprate.github.io/monero-book/consensus_rules/blocks.html#version-and-vote
    pub fn check_block_version_vote(&self, block_hf_info: &BlockHFInfo) -> bool {
        self == &block_hf_info.version && &block_hf_info.vote >= self
    }
}

/// A struct holding the current voting state of the blockchain.
#[derive(Debug, Default, Clone)]
struct HFVotes {
    votes: [u64; NUMB_OF_HARD_FORKS],
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
    /// Add votes for a hard-fork
    pub fn add_votes_for_hf(&mut self, hf: &HardFork, votes: u64) {
        self.votes[*hf as usize - 1] += votes;
    }

    /// Add a vote for a hard-fork.
    pub fn add_vote_for_hf(&mut self, hf: &HardFork) {
        self.add_votes_for_hf(hf, 1)
    }

    /// Remove a vote for a hard-fork.
    pub fn remove_vote_for_hf(&mut self, hf: &HardFork) {
        self.votes[*hf as usize - 1] -= 1;
    }

    /// Returns the total votes for a hard-fork.
    ///
    /// https://cuprate.github.io/monero-docs/consensus_rules/hardforks.html#accepting-a-fork
    pub fn votes_for_hf(&self, hf: &HardFork) -> u64 {
        self.votes[*hf as usize - 1..].iter().sum()
    }

    /// Returns the total amount of votes being tracked
    pub fn total_votes(&self) -> u64 {
        self.votes.iter().sum()
    }
}

/// A struct that keeps track of the current hard-fork and current votes.
#[derive(Debug, Clone)]
pub struct HardForkState {
    current_hardfork: HardFork,
    next_hardfork: Option<HardFork>,

    config: HardForkConfig,
    votes: HFVotes,

    last_height: u64,
}

impl HardForkState {
    pub async fn init<D: Database + Clone>(
        config: HardForkConfig,
        mut database: D,
    ) -> Result<Self, ConsensusError> {
        let DatabaseResponse::ChainHeight(chain_height, _) = database
            .ready()
            .await?
            .call(DatabaseRequest::ChainHeight)
            .await?
        else {
            panic!("Database sent incorrect response")
        };

        let hfs = HardForkState::init_from_chain_height(chain_height, config, database).await?;

        Ok(hfs)
    }

    #[instrument(name = "init_hardfork_state", skip(config, database), level = "info")]
    pub async fn init_from_chain_height<D: Database + Clone>(
        chain_height: u64,
        config: HardForkConfig,
        mut database: D,
    ) -> Result<Self, ConsensusError> {
        tracing::info!("Initializing hard-fork state this may take a while.");

        let block_start = chain_height.saturating_sub(config.window);

        let votes = get_votes_in_range(database.clone(), block_start..chain_height).await?;

        if chain_height > config.window {
            debug_assert_eq!(votes.total_votes(), config.window)
        }

        let DatabaseResponse::BlockExtendedHeader(ext_header) = database
            .ready()
            .await?
            .call(DatabaseRequest::BlockExtendedHeader(
                (chain_height - 1).into(),
            ))
            .await?
        else {
            panic!("Database sent incorrect response!");
        };

        let current_hardfork = ext_header.version;

        let next_hardfork = current_hardfork.next_fork();

        let mut hfs = HardForkState {
            config,
            current_hardfork,
            next_hardfork,
            votes,
            last_height: chain_height - 1,
        };

        hfs.check_set_new_hf();

        tracing::info!(
            "Initialized Hfs, current fork: {:?}, {}",
            hfs.current_hardfork,
            hfs.votes
        );

        Ok(hfs)
    }

    pub async fn new_block<D: Database>(
        &mut self,
        vote: HardFork,
        height: u64,
        mut database: D,
    ) -> Result<(), ConsensusError> {
        assert_eq!(self.last_height + 1, height);
        self.last_height += 1;

        tracing::debug!(
            "Accounting for new blocks vote, height: {}, vote: {:?}",
            self.last_height,
            vote
        );

        self.votes.add_vote_for_hf(&vote);

        for height_to_remove in
            (self.config.window..self.votes.total_votes()).map(|offset| height - offset)
        {
            let DatabaseResponse::BlockExtendedHeader(ext_header) = database
                .ready()
                .await?
                .call(DatabaseRequest::BlockExtendedHeader(
                    height_to_remove.into(),
                ))
                .await?
            else {
                panic!("Database sent incorrect response!");
            };

            tracing::debug!(
                "Removing block {} vote ({:?}) as they have left the window",
                height_to_remove,
                ext_header.vote
            );

            self.votes.remove_vote_for_hf(&ext_header.vote);
        }

        if height > self.config.window {
            debug_assert_eq!(self.votes.total_votes(), self.config.window);
        }

        self.check_set_new_hf();
        Ok(())
    }

    /// Checks if the next hard-fork should be activated and activates it if it should.
    ///
    /// https://cuprate.github.io/monero-docs/consensus_rules/hardforks.html#accepting-a-fork
    fn check_set_new_hf(&mut self) {
        while let Some(new_hf) = self.next_hardfork {
            let hf_info = self.config.fork_info(&new_hf);
            if self.last_height + 1 >= hf_info.height
                && self.votes.votes_for_hf(&new_hf)
                    >= votes_needed(hf_info.threshold, self.config.window)
            {
                self.set_hf(new_hf);
            } else {
                return;
            }
        }
    }

    /// Sets a new hard-fork.
    fn set_hf(&mut self, new_hf: HardFork) {
        self.next_hardfork = new_hf.next_fork();
        self.current_hardfork = new_hf;
    }

    pub fn current_hardfork(&self) -> HardFork {
        self.current_hardfork
    }
}

/// Returns the votes needed for this fork.
///
/// https://cuprate.github.io/monero-docs/consensus_rules/hardforks.html#accepting-a-fork
pub fn votes_needed(threshold: u64, window: u64) -> u64 {
    (threshold * window + 99) / 100
}

#[instrument(name = "get_votes", skip(database))]
async fn get_votes_in_range<D: Database>(
    database: D,
    block_heights: Range<u64>,
) -> Result<HFVotes, ConsensusError> {
    let mut votes = HFVotes::default();

    let DatabaseResponse::BlockExtendedHeaderInRange(vote_list) = database
        .oneshot(DatabaseRequest::BlockExtendedHeaderInRange(block_heights))
        .await?
    else {
        panic!("Database sent incorrect response!");
    };

    for hf_info in vote_list.into_iter() {
        votes.add_vote_for_hf(&hf_info.vote);
    }

    Ok(votes)
}

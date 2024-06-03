use std::ops::Range;

use tower::ServiceExt;
use tracing::instrument;

use cuprate_consensus_rules::{HFVotes, HFsInfo, HardFork};
use cuprate_types::blockchain::{BCReadRequest, BCResponse};

use crate::{Database, ExtendedConsensusError};

/// The default amount of hard-fork votes to track to decide on activation of a hard-fork.
///
/// ref: <https://cuprate.github.io/monero-docs/consensus_rules/hardforks.html#accepting-a-fork>
const DEFAULT_WINDOW_SIZE: u64 = 10080; // supermajority window check length - a week

/// Configuration for hard-forks.
///
#[derive(Debug, Clone)]
pub struct HardForkConfig {
    /// The network we are on.
    pub(crate) info: HFsInfo,
    /// The amount of votes we are taking into account to decide on a fork activation.
    pub(crate) window: u64,
}

impl HardForkConfig {
    /// Config for main-net.
    pub const fn main_net() -> HardForkConfig {
        Self {
            info: HFsInfo::main_net(),
            window: DEFAULT_WINDOW_SIZE,
        }
    }

    /// Config for stage-net.
    pub const fn stage_net() -> HardForkConfig {
        Self {
            info: HFsInfo::stage_net(),
            window: DEFAULT_WINDOW_SIZE,
        }
    }

    /// Config for test-net.
    pub const fn test_net() -> HardForkConfig {
        Self {
            info: HFsInfo::test_net(),
            window: DEFAULT_WINDOW_SIZE,
        }
    }
}

/// A struct that keeps track of the current hard-fork and current votes.
#[derive(Debug, Clone)]
pub struct HardForkState {
    /// The current active hard-fork.
    pub(crate) current_hardfork: HardFork,

    /// The hard-fork config.
    pub(crate) config: HardForkConfig,
    /// The votes in the current window.
    pub(crate) votes: HFVotes,

    /// The last block height accounted for.
    pub(crate) last_height: u64,
}

impl HardForkState {
    /// Initialize the [`HardForkState`] from the specified chain height.
    #[instrument(name = "init_hardfork_state", skip(config, database), level = "info")]
    pub async fn init_from_chain_height<D: Database + Clone>(
        chain_height: u64,
        config: HardForkConfig,
        mut database: D,
    ) -> Result<Self, ExtendedConsensusError> {
        tracing::info!("Initializing hard-fork state this may take a while.");

        let block_start = chain_height.saturating_sub(config.window);

        let votes = get_votes_in_range(
            database.clone(),
            block_start..chain_height,
            usize::try_from(config.window).unwrap(),
        )
        .await?;

        if chain_height > config.window {
            debug_assert_eq!(votes.total_votes(), config.window)
        }

        let BCResponse::BlockExtendedHeader(ext_header) = database
            .ready()
            .await?
            .call(BCReadRequest::BlockExtendedHeader(chain_height - 1))
            .await?
        else {
            panic!("Database sent incorrect response!");
        };

        let current_hardfork =
            HardFork::from_version(ext_header.version).expect("Stored block has invalid hardfork");

        let mut hfs = HardForkState {
            config,
            current_hardfork,
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

    /// Add a new block to the cache.
    pub fn new_block(&mut self, vote: HardFork, height: u64) {
        // We don't _need_ to take in `height` but it's for safety, so we don't silently loose track
        // of blocks.
        assert_eq!(self.last_height + 1, height);
        self.last_height += 1;

        tracing::debug!(
            "Accounting for new blocks vote, height: {}, vote: {:?}",
            self.last_height,
            vote
        );

        // This function remove votes outside the window as well.
        self.votes.add_vote_for_hf(&vote);

        if height > self.config.window {
            debug_assert_eq!(self.votes.total_votes(), self.config.window);
        }

        self.check_set_new_hf();
    }

    /// Checks if the next hard-fork should be activated and activates it if it should.
    ///
    /// https://cuprate.github.io/monero-docs/consensus_rules/hardforks.html#accepting-a-fork
    fn check_set_new_hf(&mut self) {
        self.current_hardfork = self.votes.current_fork(
            &self.current_hardfork,
            self.last_height + 1,
            self.config.window,
            &self.config.info,
        );
    }

    /// Returns the current hard-fork.
    pub fn current_hardfork(&self) -> HardFork {
        self.current_hardfork
    }
}

/// Returns the block votes for blocks in the specified range.
#[instrument(name = "get_votes", skip(database))]
async fn get_votes_in_range<D: Database>(
    database: D,
    block_heights: Range<u64>,
    window_size: usize,
) -> Result<HFVotes, ExtendedConsensusError> {
    let mut votes = HFVotes::new(window_size);

    let BCResponse::BlockExtendedHeaderInRange(vote_list) = database
        .oneshot(BCReadRequest::BlockExtendedHeaderInRange(block_heights))
        .await?
    else {
        panic!("Database sent incorrect response!");
    };

    for hf_info in vote_list.into_iter() {
        votes.add_vote_for_hf(&HardFork::from_vote(hf_info.vote));
    }

    Ok(votes)
}

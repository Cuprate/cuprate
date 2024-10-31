//! Difficulty Module
//!
//! This module handles keeping track of the data required to calculate block difficulty.
//! This data is currently the cumulative difficulty of each block and its timestamp.
//!
//! The timestamps are also used in other consensus rules so instead of duplicating the same
//! data in a different cache, the timestamps needed are retrieved from here.
//!
use std::{collections::VecDeque, ops::Range};

use tower::ServiceExt;
use tracing::instrument;

use cuprate_helper::num::median;
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    Chain,
};

use crate::{ContextCacheError, Database, HardFork};

/// The amount of blocks we account for to calculate difficulty
const DIFFICULTY_WINDOW: usize = 720;
/// The proportion of blocks we remove from the [`DIFFICULTY_WINDOW`]. When the window
/// if 720 this means that 60 blocks are removed from the ends of the window so 120
/// blocks removed in total.
const DIFFICULTY_CUT: usize = 60;
/// The amount of blocks we add onto the window before doing any calculations so that the
/// difficulty lags by this amount of blocks
const DIFFICULTY_LAG: usize = 15;

/// Configuration for the difficulty cache.
///
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DifficultyCacheConfig {
    pub window: usize,
    pub cut: usize,
    pub lag: usize,
}

impl DifficultyCacheConfig {
    /// Create a new difficulty cache config.
    ///
    /// # Notes
    /// You probably do not need this, use [`DifficultyCacheConfig::main_net`] instead.
    pub const fn new(window: usize, cut: usize, lag: usize) -> Self {
        Self { window, cut, lag }
    }

    /// Returns the total amount of blocks we need to track to calculate difficulty
    pub const fn total_block_count(&self) -> usize {
        self.window + self.lag
    }

    /// The amount of blocks we account for after removing the outliers.
    pub const fn accounted_window_len(&self) -> usize {
        self.window - 2 * self.cut
    }

    /// Returns the config needed for [`Mainnet`](cuprate_helper::network::Network::Mainnet). This is also the
    /// config for all other current networks.
    pub const fn main_net() -> Self {
        Self {
            window: DIFFICULTY_WINDOW,
            cut: DIFFICULTY_CUT,
            lag: DIFFICULTY_LAG,
        }
    }
}

/// This struct is able to calculate difficulties from blockchain information.
///
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DifficultyCache {
    /// The list of timestamps in the window.
    pub timestamps: VecDeque<u64>,
    /// The current cumulative difficulty of the chain.
    pub cumulative_difficulties: VecDeque<u128>,
    /// The last height we accounted for.
    pub last_accounted_height: usize,
    /// The config
    pub config: DifficultyCacheConfig,
}

impl DifficultyCache {
    /// Initialize the difficulty cache from the specified chain height.
    #[instrument(name = "init_difficulty_cache", level = "info", skip(database, config))]
    pub async fn init_from_chain_height<D: Database + Clone>(
        chain_height: usize,
        config: DifficultyCacheConfig,
        database: D,
        chain: Chain,
    ) -> Result<Self, ContextCacheError> {
        tracing::info!("Initializing difficulty cache this may take a while.");

        let mut block_start = chain_height.saturating_sub(config.total_block_count());

        // skip the genesis block.
        if block_start == 0 {
            block_start = 1;
        }

        let (timestamps, cumulative_difficulties) =
            get_blocks_in_pow_info(database.clone(), block_start..chain_height, chain).await?;

        debug_assert_eq!(timestamps.len(), chain_height - block_start);

        tracing::info!(
            "Current chain height: {}, accounting for {} blocks timestamps",
            chain_height,
            timestamps.len()
        );

        let diff = Self {
            timestamps,
            cumulative_difficulties,
            last_accounted_height: chain_height - 1,
            config,
        };

        Ok(diff)
    }

    /// Pop some blocks from the top of the cache.
    ///
    /// The cache will be returned to the state it would have been in `numb_blocks` ago.
    ///
    /// # Invariant
    ///
    /// This _must_ only be used on a main-chain cache.
    #[instrument(name = "pop_blocks_diff_cache", skip_all, fields(numb_blocks = numb_blocks))]
    pub async fn pop_blocks_main_chain<D: Database + Clone>(
        &mut self,
        numb_blocks: usize,
        database: D,
    ) -> Result<(), ContextCacheError> {
        let Some(retained_blocks) = self.timestamps.len().checked_sub(numb_blocks) else {
            // More blocks to pop than we have in the cache, so just restart a new cache.
            *self = Self::init_from_chain_height(
                self.last_accounted_height - numb_blocks + 1,
                self.config,
                database,
                Chain::Main,
            )
            .await?;

            return Ok(());
        };

        let current_chain_height = self.last_accounted_height + 1;

        let mut new_start_height = current_chain_height
            .saturating_sub(self.config.total_block_count())
            .saturating_sub(numb_blocks);

        // skip the genesis block.
        if new_start_height == 0 {
            new_start_height = 1;
        }

        let (mut timestamps, mut cumulative_difficulties) = get_blocks_in_pow_info(
            database,
            new_start_height
                // current_chain_height - self.timestamps.len() blocks are already in the cache.
                ..(current_chain_height - self.timestamps.len()),
            Chain::Main,
        )
        .await?;

        self.timestamps.drain(retained_blocks..);
        self.cumulative_difficulties.drain(retained_blocks..);
        timestamps.append(&mut self.timestamps);
        cumulative_difficulties.append(&mut self.cumulative_difficulties);

        self.timestamps = timestamps;
        self.cumulative_difficulties = cumulative_difficulties;
        self.last_accounted_height -= numb_blocks;

        assert_eq!(self.timestamps.len(), self.cumulative_difficulties.len());

        Ok(())
    }

    /// Add a new block to the difficulty cache.
    pub fn new_block(&mut self, height: usize, timestamp: u64, cumulative_difficulty: u128) {
        assert_eq!(self.last_accounted_height + 1, height);
        self.last_accounted_height += 1;

        tracing::debug!(
            "Accounting for new blocks timestamp ({timestamp}) and cumulative_difficulty ({cumulative_difficulty})",
        );

        self.timestamps.push_back(timestamp);
        self.cumulative_difficulties
            .push_back(cumulative_difficulty);

        if self.timestamps.len() > self.config.total_block_count() {
            self.timestamps.pop_front();
            self.cumulative_difficulties.pop_front();
        }
    }

    /// Returns the required difficulty for the next block.
    ///
    /// See: <https://cuprate.github.io/monero-book/consensus_rules/blocks/difficulty.html#calculating-difficulty>
    pub fn next_difficulty(&self, hf: HardFork) -> u128 {
        next_difficulty(
            &self.config,
            &self.timestamps,
            &self.cumulative_difficulties,
            hf,
        )
    }

    /// Returns the difficulties for multiple next blocks, using the provided timestamps and hard-forks when needed.
    ///
    /// The first difficulty will be the same as the difficulty from [`DifficultyCache::next_difficulty`] after that the
    /// first timestamp and hf will be applied to the cache and the difficulty from that will be added to the list.
    ///
    /// After all timestamps and hfs have been dealt with the cache will be returned back to its original state and the
    /// difficulties will be returned.
    pub fn next_difficulties(
        &self,
        blocks: Vec<(u64, HardFork)>,
        current_hf: HardFork,
    ) -> Vec<u128> {
        let mut timestamps = self.timestamps.clone();
        let mut cumulative_difficulties = self.cumulative_difficulties.clone();

        let mut difficulties = Vec::with_capacity(blocks.len() + 1);

        difficulties.push(self.next_difficulty(current_hf));

        for (new_timestamp, hf) in blocks {
            timestamps.push_back(new_timestamp);

            let last_cum_diff = cumulative_difficulties.back().copied().unwrap_or(1);
            cumulative_difficulties.push_back(last_cum_diff + *difficulties.last().unwrap());

            if timestamps.len() > self.config.total_block_count() {
                timestamps.pop_front().unwrap();
                cumulative_difficulties.pop_front().unwrap();
            }

            difficulties.push(next_difficulty(
                &self.config,
                &timestamps,
                &cumulative_difficulties,
                hf,
            ));
        }

        difficulties
    }

    /// Returns the median timestamp over the last `numb_blocks`, including the genesis block if the block height is low enough.
    ///
    /// Will return [`None`] if there aren't enough blocks.
    pub fn median_timestamp(&self, numb_blocks: usize) -> Option<u64> {
        let mut timestamps = if self.last_accounted_height + 1 == numb_blocks {
            // if the chain height is equal to `numb_blocks` add the genesis block.
            // otherwise if the chain height is less than `numb_blocks` None is returned
            // and if it's more it would be excluded from calculations.
            let mut timestamps = self.timestamps.clone();
            // all genesis blocks have a timestamp of 0.
            // https://cuprate.github.io/monero-book/consensus_rules/genesis_block.html
            timestamps.push_front(0);
            timestamps.into()
        } else {
            self.timestamps
                .range(self.timestamps.len().checked_sub(numb_blocks)?..)
                .copied()
                .collect::<Vec<_>>()
        };
        timestamps.sort_unstable();
        debug_assert_eq!(timestamps.len(), numb_blocks);

        Some(median(&timestamps))
    }

    /// Returns the cumulative difficulty of the chain.
    pub fn cumulative_difficulty(&self) -> u128 {
        // the genesis block has a difficulty of 1
        self.cumulative_difficulties.back().copied().unwrap_or(1)
    }

    /// Returns the top block's timestamp, returns [`None`] if the top block is the genesis block.
    pub fn top_block_timestamp(&self) -> Option<u64> {
        self.timestamps.back().copied()
    }
}

/// Calculates the next difficulty with the inputted `config/timestamps/cumulative_difficulties`.
fn next_difficulty(
    config: &DifficultyCacheConfig,
    timestamps: &VecDeque<u64>,
    cumulative_difficulties: &VecDeque<u128>,
    hf: HardFork,
) -> u128 {
    if timestamps.len() <= 1 {
        return 1;
    }

    let mut timestamps = timestamps.clone();

    if timestamps.len() > config.window {
        // remove the lag.
        timestamps.drain(config.window..);
    };
    let timestamps_slice = timestamps.make_contiguous();

    let (window_start, window_end) = get_window_start_and_end(
        timestamps_slice.len(),
        config.accounted_window_len(),
        config.window,
    );

    // We don't sort the whole timestamp list
    let mut time_span = u128::from(
        *timestamps_slice.select_nth_unstable(window_end - 1).1
            - *timestamps_slice.select_nth_unstable(window_start).1,
    );

    let windowed_work =
        cumulative_difficulties[window_end - 1] - cumulative_difficulties[window_start];

    if time_span == 0 {
        time_span = 1;
    }

    // TODO: do checked operations here and unwrap so we don't silently overflow?
    (windowed_work * u128::from(hf.block_time().as_secs()) + time_span - 1) / time_span
}

/// Get the start and end of the window to calculate difficulty.
fn get_window_start_and_end(
    window_len: usize,
    accounted_window: usize,
    window: usize,
) -> (usize, usize) {
    debug_assert!(window > accounted_window);

    let window_len = if window_len > window {
        window
    } else {
        window_len
    };

    if window_len <= accounted_window {
        (0, window_len)
    } else {
        let start = (window_len - (accounted_window) + 1) / 2;
        (start, start + accounted_window)
    }
}

/// Returns the timestamps and cumulative difficulty for the blocks with heights in the specified range.
#[instrument(name = "get_blocks_timestamps", skip(database), level = "info")]
async fn get_blocks_in_pow_info<D: Database + Clone>(
    database: D,
    block_heights: Range<usize>,
    chain: Chain,
) -> Result<(VecDeque<u64>, VecDeque<u128>), ContextCacheError> {
    tracing::info!("Getting blocks timestamps");

    let BlockchainResponse::BlockExtendedHeaderInRange(ext_header) = database
        .oneshot(BlockchainReadRequest::BlockExtendedHeaderInRange(
            block_heights,
            chain,
        ))
        .await?
    else {
        panic!("Database sent incorrect response");
    };

    Ok(ext_header
        .into_iter()
        .map(|info| (info.timestamp, info.cumulative_difficulty))
        .unzip())
}

use std::{collections::VecDeque, ops::Range};

use tower::ServiceExt;
use tracing::instrument;

use cuprate_helper::num::median;

use crate::{Database, DatabaseRequest, DatabaseResponse, ExtendedConsensusError, HardFork};

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
#[derive(Debug, Clone)]
pub struct DifficultyCacheConfig {
    pub(crate) window: usize,
    pub(crate) cut: usize,
    pub(crate) lag: usize,
}

impl DifficultyCacheConfig {
    pub const fn new(window: usize, cut: usize, lag: usize) -> DifficultyCacheConfig {
        DifficultyCacheConfig { window, cut, lag }
    }

    /// Returns the total amount of blocks we need to track to calculate difficulty
    pub fn total_block_count(&self) -> u64 {
        (self.window + self.lag).try_into().unwrap()
    }

    /// The amount of blocks we account for after removing the outliers.
    pub fn accounted_window_len(&self) -> usize {
        self.window - 2 * self.cut
    }

    pub fn main_net() -> DifficultyCacheConfig {
        DifficultyCacheConfig {
            window: DIFFICULTY_WINDOW,
            cut: DIFFICULTY_CUT,
            lag: DIFFICULTY_LAG,
        }
    }
}

/// This struct is able to calculate difficulties from blockchain information.
///
#[derive(Debug, Clone)]
pub(crate) struct DifficultyCache {
    /// The list of timestamps in the window.
    /// len <= [`DIFFICULTY_BLOCKS_COUNT`]
    pub(crate) timestamps: VecDeque<u64>,
    /// The current cumulative difficulty of the chain.
    pub(crate) cumulative_difficulties: VecDeque<u128>,
    /// The last height we accounted for.
    pub(crate) last_accounted_height: u64,
    /// The config
    pub(crate) config: DifficultyCacheConfig,
}

impl DifficultyCache {
    #[instrument(name = "init_difficulty_cache", level = "info", skip(database, config))]
    pub async fn init_from_chain_height<D: Database + Clone>(
        chain_height: u64,
        config: DifficultyCacheConfig,
        database: D,
    ) -> Result<Self, ExtendedConsensusError> {
        tracing::info!("Initializing difficulty cache this may take a while.");

        let mut block_start = chain_height.saturating_sub(config.total_block_count());

        // skip the genesis block.
        if block_start == 0 {
            block_start = 1;
        }

        let (timestamps, cumulative_difficulties) =
            get_blocks_in_pow_info(database.clone(), block_start..chain_height).await?;

        tracing::info!(
            "Current chain height: {}, accounting for {} blocks timestamps",
            chain_height,
            timestamps.len()
        );

        let diff = DifficultyCache {
            timestamps,
            cumulative_difficulties,
            last_accounted_height: chain_height - 1,
            config,
        };

        Ok(diff)
    }

    pub fn new_block(&mut self, height: u64, timestamp: u64, cumulative_difficulty: u128) {
        assert_eq!(self.last_accounted_height + 1, height);
        self.last_accounted_height += 1;

        self.timestamps.push_back(timestamp);
        self.cumulative_difficulties
            .push_back(cumulative_difficulty);
        if u64::try_from(self.timestamps.len()).unwrap() > self.config.total_block_count() {
            self.timestamps.pop_front();
            self.cumulative_difficulties.pop_front();
        }
    }

    /// Returns the required difficulty for the next block.
    ///
    /// See: https://cuprate.github.io/monero-book/consensus_rules/blocks/difficulty.html#calculating-difficulty
    pub fn next_difficulty(&self, hf: &HardFork) -> u128 {
        if self.timestamps.len() <= 1 {
            return 1;
        }

        let mut timestamps = self.timestamps.clone();
        if timestamps.len() > self.config.window {
            // remove the lag.
            timestamps.drain(self.config.window..);
        };
        let timestamps_slice = timestamps.make_contiguous();

        let (window_start, window_end) = get_window_start_and_end(
            timestamps_slice.len(),
            self.config.accounted_window_len(),
            self.config.window,
        );

        // We don't sort the whole timestamp list
        let mut time_span = u128::from(
            *timestamps_slice.select_nth_unstable(window_end - 1).1
                - *timestamps_slice.select_nth_unstable(window_start).1,
        );

        let windowed_work = self.cumulative_difficulties[window_end - 1]
            - self.cumulative_difficulties[window_start];

        if time_span == 0 {
            time_span = 1;
        }

        // TODO: do checked operations here and unwrap so we don't silently overflow?
        (windowed_work * hf.block_time().as_secs() as u128 + time_span - 1) / time_span
    }

    pub fn next_difficulties(
        &mut self,
        blocks: Vec<(u64, HardFork)>,
        current_hf: &HardFork,
    ) -> Vec<u128> {
        let new_timestamps_len = blocks.len();
        let initial_len = self.timestamps.len();

        let mut difficulties = Vec::with_capacity(blocks.len() + 1);

        difficulties.push(self.next_difficulty(current_hf));

        let mut diff_info_popped = Vec::new();

        for (new_timestamp, hf) in blocks {
            self.timestamps.push_back(new_timestamp);
            self.cumulative_difficulties
                .push_back(self.cumulative_difficulty() + *difficulties.last().unwrap());
            if u64::try_from(self.timestamps.len()).unwrap() > self.config.total_block_count() {
                diff_info_popped.push((
                    self.timestamps.pop_front().unwrap(),
                    self.cumulative_difficulties.pop_front().unwrap(),
                ));
            }

            difficulties.push(self.next_difficulty(&hf));
        }

        self.cumulative_difficulties.drain(
            self.cumulative_difficulties
                .len()
                .saturating_sub(new_timestamps_len)..,
        );
        self.timestamps
            .drain(self.timestamps.len().saturating_sub(new_timestamps_len)..);

        for (timestamp, cum_dif) in diff_info_popped.into_iter().take(initial_len).rev() {
            self.timestamps.push_front(timestamp);
            self.cumulative_difficulties.push_front(cum_dif);
        }

        difficulties
    }

    /// Returns the median timestamp over the last `numb_blocks`, including the genesis block if the block height is low enough.
    ///
    /// Will return [`None`] if there aren't enough blocks.
    pub fn median_timestamp(&self, numb_blocks: usize) -> Option<u64> {
        let mut timestamps =
            if self.last_accounted_height + 1 == u64::try_from(numb_blocks).unwrap() {
                // if the chain height is equal to `numb_blocks` add the genesis block.
                // otherwise if the chain height is less than `numb_blocks` None is returned
                // and if its more than it would be excluded from calculations.
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

    pub fn top_block_timestamp(&self) -> Option<u64> {
        self.timestamps.back().copied()
    }
}

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

#[instrument(name = "get_blocks_timestamps", skip(database), level = "info")]
async fn get_blocks_in_pow_info<D: Database + Clone>(
    database: D,
    block_heights: Range<u64>,
) -> Result<(VecDeque<u64>, VecDeque<u128>), ExtendedConsensusError> {
    tracing::info!("Getting blocks timestamps");

    let DatabaseResponse::BlockExtendedHeaderInRange(ext_header) = database
        .oneshot(DatabaseRequest::BlockExtendedHeaderInRange(block_heights))
        .await?
    else {
        panic!("Database sent incorrect response");
    };

    Ok(ext_header
        .into_iter()
        .map(|info| (info.timestamp, info.cumulative_difficulty))
        .unzip())
}

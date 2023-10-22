use std::{collections::VecDeque, ops::Range};

use tower::ServiceExt;
use tracing::instrument;

use crate::{
    helper::median, ConsensusError, Database, DatabaseRequest, DatabaseResponse, HardFork,
};

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
    window: usize,
    cut: usize,
    lag: usize,
}

impl DifficultyCacheConfig {
    pub fn new(window: usize, cut: usize, lag: usize) -> DifficultyCacheConfig {
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
pub struct DifficultyCache {
    /// The list of timestamps in the window.
    /// len <= [`DIFFICULTY_BLOCKS_COUNT`]
    timestamps: VecDeque<u64>,
    /// The work done in the [`DIFFICULTY_ACCOUNTED_WINDOW_LEN`] window, this is an optimisation
    /// so we don't need to keep track of cumulative difficulties as well as timestamps.
    windowed_work: u128,
    /// The current cumulative difficulty of the chain.
    cumulative_difficulty: u128,
    /// The last height we accounted for.
    last_accounted_height: u64,
    /// The config
    config: DifficultyCacheConfig,
}

impl DifficultyCache {
    pub async fn init<D: Database + Clone>(
        config: DifficultyCacheConfig,
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

        DifficultyCache::init_from_chain_height(chain_height, config, database).await
    }

    #[instrument(name = "init_difficulty_cache", level = "info", skip(database, config))]
    pub async fn init_from_chain_height<D: Database + Clone>(
        chain_height: u64,
        config: DifficultyCacheConfig,
        database: D,
    ) -> Result<Self, ConsensusError> {
        tracing::info!("Initializing difficulty cache this may take a while.");

        let mut block_start = chain_height.saturating_sub(config.total_block_count());

        // skip the genesis block.
        if block_start == 0 {
            block_start = 1;
        }

        let timestamps =
            get_blocks_in_range_timestamps(database.clone(), block_start..chain_height).await?;

        let mut diff = DifficultyCache {
            timestamps,
            windowed_work: 0,
            cumulative_difficulty: 0,
            last_accounted_height: chain_height - 1,
            config,
        };

        diff.update_windowed_work(database).await?;

        tracing::info!(
            "Current chain height: {}, accounting for {} blocks timestamps",
            chain_height,
            diff.timestamps.len()
        );

        Ok(diff)
    }

    pub async fn new_block<D: Database>(
        &mut self,
        height: u64,
        timestamp: u64,
        database: D,
    ) -> Result<(), ConsensusError> {
        assert_eq!(self.last_accounted_height + 1, height);
        self.last_accounted_height += 1;

        self.timestamps.pop_front();
        self.timestamps.push_back(timestamp);

        self.update_windowed_work(database).await?;

        Ok(())
    }

    async fn update_windowed_work<D: Database>(
        &mut self,
        mut database: D,
    ) -> Result<(), ConsensusError> {
        if self.last_accounted_height == 0 {
            return Ok(());
        }

        let mut block_start =
            (self.last_accounted_height + 1).saturating_sub(self.config.total_block_count());

        // skip the genesis block
        if block_start == 0 {
            block_start = 1;
        }

        let (start, end) =
            get_window_start_and_end(self.timestamps.len(), self.config.accounted_window_len());

        let low_cumulative_difficulty = get_block_cum_diff(
            &mut database,
            block_start + TryInto::<u64>::try_into(start).unwrap(),
        )
        .await?;

        let high_cumulative_difficulty = get_block_cum_diff(
            &mut database,
            block_start + TryInto::<u64>::try_into(end).unwrap() - 1,
        )
        .await?;

        let chain_cumulative_difficulty =
            get_block_cum_diff(&mut database, self.last_accounted_height).await?;

        self.cumulative_difficulty = chain_cumulative_difficulty;
        self.windowed_work = high_cumulative_difficulty - low_cumulative_difficulty;
        Ok(())
    }

    /// Returns the required difficulty for the next block.
    ///
    /// See: https://cuprate.github.io/monero-book/consensus_rules/blocks/difficulty.html#calculating-difficulty
    pub fn next_difficulty(&self, hf: &HardFork) -> u128 {
        if self.timestamps.len() <= 1 {
            return 1;
        }

        let mut sorted_timestamps = self.timestamps.clone();
        if sorted_timestamps.len() > DIFFICULTY_WINDOW {
            sorted_timestamps.drain(DIFFICULTY_WINDOW..);
        };
        sorted_timestamps.make_contiguous().sort_unstable();

        let (window_start, window_end) =
            get_window_start_and_end(sorted_timestamps.len(), self.config.accounted_window_len());

        let mut time_span =
            u128::from(sorted_timestamps[window_end - 1] - sorted_timestamps[window_start]);

        if time_span == 0 {
            time_span = 1;
        }

        (self.windowed_work * hf.block_time().as_secs() as u128 + time_span - 1) / time_span
    }

    /// Returns the median timestamp over the last `numb_blocks`.
    ///
    /// Will panic if `numb_blocks` is larger than amount of blocks in the cache.
    pub fn median_timestamp(&self, numb_blocks: usize) -> u64 {
        median(
            &self
                .timestamps
                .range(self.timestamps.len().checked_sub(numb_blocks).unwrap()..)
                .copied()
                .collect::<Vec<_>>(),
        )
    }

    /// Returns the cumulative difficulty of the chain.
    pub fn cumulative_difficulty(&self) -> u128 {
        self.cumulative_difficulty
    }

    pub fn top_block_timestamp(&self) -> Option<u64> {
        self.timestamps.back().copied()
    }
}

fn get_window_start_and_end(window_len: usize, accounted_window: usize) -> (usize, usize) {
    let window_len = if window_len > DIFFICULTY_WINDOW {
        DIFFICULTY_WINDOW
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
async fn get_blocks_in_range_timestamps<D: Database + Clone>(
    database: D,
    block_heights: Range<u64>,
) -> Result<VecDeque<u64>, ConsensusError> {
    tracing::info!("Getting blocks timestamps");

    let DatabaseResponse::BlockExtendedHeaderInRange(ext_header) = database
        .oneshot(DatabaseRequest::BlockExtendedHeaderInRange(block_heights))
        .await?
    else {
        panic!("Database sent incorrect response");
    };

    Ok(ext_header.into_iter().map(|info| info.timestamp).collect())
}

async fn get_block_cum_diff<D: Database>(database: D, height: u64) -> Result<u128, ConsensusError> {
    let DatabaseResponse::BlockExtendedHeader(ext_header) = database
        .oneshot(DatabaseRequest::BlockExtendedHeader(height.into()))
        .await?
    else {
        panic!("Database service sent incorrect response!");
    };
    Ok(ext_header.cumulative_difficulty)
}

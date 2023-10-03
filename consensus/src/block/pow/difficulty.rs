

use std::ops::Range;
use tower::ServiceExt;
use tracing::instrument;

use crate::{hardforks::HardFork, ConsensusError, Database, DatabaseRequest, DatabaseResponse};

/// The amount of blocks we account for to calculate difficulty
const DIFFICULTY_WINDOW: usize = 720;
/// The proportion of blocks we remove from the [`DIFFICULTY_WINDOW`]. When the window
/// if 720 this means that 60 blocks are removed from the ends of the window so 120
/// blocks removed in total.
const DIFFICULTY_CUT: usize = 60;
/// The amount of blocks we add onto the window before doing any calculations so that the
/// difficulty lags by this amount of blocks
const DIFFICULTY_LAG: usize = 15;
/// The total amount of blocks we need to track to calculate difficulty
const DIFFICULTY_BLOCKS_COUNT: u64 = (DIFFICULTY_WINDOW + DIFFICULTY_LAG) as u64;
/// The amount of blocks we account for after removing the outliers.
const DIFFICULTY_ACCOUNTED_WINDOW_LEN: usize = DIFFICULTY_WINDOW - 2 * DIFFICULTY_CUT;

/// This struct is able to calculate difficulties from blockchain information.
#[derive(Debug, Clone)]
pub struct DifficultyCache {
    /// The list of timestamps in the window.
    /// len <= [`DIFFICULTY_BLOCKS_COUNT`]
    timestamps: Vec<u64>,
    /// The work done in the [`DIFFICULTY_ACCOUNTED_WINDOW_LEN`] window, this is an optimisation
    /// so we don't need to keep track of cumulative difficulties as well as timestamps.
    windowed_work: u128,
    /// The last height we accounted for.
    last_accounted_height: u64,
}

impl DifficultyCache {
    pub async fn init<D: Database + Clone>(mut database: D) -> Result<Self, ConsensusError> {
        let DatabaseResponse::ChainHeight(chain_height) = database
            .ready()
            .await?
            .call(DatabaseRequest::ChainHeight)
            .await?
        else {
            panic!("Database sent incorrect response")
        };

        DifficultyCache::init_from_chain_height(chain_height, database).await
    }

    #[instrument(name = "init_difficulty_cache", level = "info", skip(database))]
    pub async fn init_from_chain_height<D: Database + Clone>(
        chain_height: u64,
        mut database: D,
    ) -> Result<Self, ConsensusError> {
        tracing::info!("Initializing difficulty cache this may take a while.");

        let mut block_start = chain_height.saturating_sub(DIFFICULTY_BLOCKS_COUNT);

        if block_start == 0 {
            block_start = 1;
        }

        let timestamps =
            get_blocks_in_range_timestamps(database.clone(), block_start..chain_height).await?;

        let mut diff = DifficultyCache {
            timestamps,
            windowed_work: 0,
            last_accounted_height: chain_height - 1,
        };

        diff.update_windowed_work(&mut database).await?;

        tracing::info!(
            "Current chain height: {}, accounting for {} blocks timestamps",
            chain_height,
            diff.timestamps.len()
        );

        Ok(diff)
    }

    pub async fn resync<D: Database + Clone>(
        &mut self,
        mut database: D,
    ) -> Result<(), ConsensusError> {
        let DatabaseResponse::ChainHeight(chain_height) = database
            .ready()
            .await?
            .call(DatabaseRequest::ChainHeight)
            .await?
        else {
            panic!("Database sent incorrect response")
        };

        // TODO: We need to handle re-orgs
        assert!(chain_height > self.last_accounted_height);

        if chain_height == self.last_accounted_height + 1 {
            return Ok(());
        }

        let mut timestamps = get_blocks_in_range_timestamps(
            database.clone(),
            self.last_accounted_height + 1..chain_height,
        )
        .await?;

        self.timestamps.append(&mut timestamps);

        self.timestamps.drain(
            0..self
                .timestamps
                .len()
                .saturating_sub(DIFFICULTY_BLOCKS_COUNT as usize),
        );

        self.last_accounted_height = chain_height - 1;

        self.update_windowed_work(database).await
    }

    async fn update_windowed_work<D: Database>(
        &mut self,
        mut database: D,
    ) -> Result<(), ConsensusError> {
        if self.last_accounted_height == 0 {
            return Ok(());
        }

        let mut block_start =
            (self.last_accounted_height + 1).saturating_sub(DIFFICULTY_BLOCKS_COUNT);

        if block_start == 0 {
            block_start = 1;
        }

        let (start, end) = get_window_start_and_end(self.timestamps.len());

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
        sorted_timestamps.sort_unstable();

        let (window_start, window_end) = get_window_start_and_end(sorted_timestamps.len());

        let mut time_span =
            u128::from(sorted_timestamps[window_end - 1] - sorted_timestamps[window_start]);

        if time_span == 0 {
            time_span = 1;
        }

        (self.windowed_work * target_time_for_hf(hf) + time_span - 1) / time_span
    }
}

fn get_window_start_and_end(window_len: usize) -> (usize, usize) {
    let window_len = if window_len > DIFFICULTY_WINDOW {
        DIFFICULTY_WINDOW
    } else {
        window_len
    };

    if window_len <= DIFFICULTY_ACCOUNTED_WINDOW_LEN {
        (0, window_len)
    } else {
        let start = (window_len - (DIFFICULTY_ACCOUNTED_WINDOW_LEN) + 1) / 2;
        (start, start + DIFFICULTY_ACCOUNTED_WINDOW_LEN)
    }
}

#[instrument(name = "get_blocks_timestamps", skip(database), level = "info")]
async fn get_blocks_in_range_timestamps<D: Database + Clone>(
    database: D,
    block_heights: Range<u64>,
) -> Result<Vec<u64>, ConsensusError> {
    tracing::info!("Getting blocks timestamps");

    let DatabaseResponse::BlockPOWInfoInRange(pow_infos) = database
        .oneshot(DatabaseRequest::BlockPOWInfoInRange(block_heights))
        .await?
    else {
        panic!("Database sent incorrect response");
    };

    Ok(pow_infos.into_iter().map(|info| info.timestamp).collect())
}

async fn get_block_cum_diff<D: Database>(database: D, height: u64) -> Result<u128, ConsensusError> {
    let DatabaseResponse::BlockPOWInfo(pow) = database
        .oneshot(DatabaseRequest::BlockPOWInfo(height.into()))
        .await?
    else {
        panic!("Database service sent incorrect response!");
    };
    Ok(pow.cumulative_difficulty)
}

fn target_time_for_hf(hf: &HardFork) -> u128 {
    match hf {
        HardFork::V1 => 60,
        _ => 120,
    }
}

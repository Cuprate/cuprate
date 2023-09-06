use futures::stream::FuturesOrdered;
use futures::{StreamExt, TryFutureExt};
use std::ops::Range;
use tower::ServiceExt;
use tracing::instrument;

use crate::{hardforks::HardFork, Database, DatabaseRequest, DatabaseResponse, Error};

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
#[derive(Debug)]
pub struct DifficultyCalculator {
    /// The list of timestamps in the window.
    /// len <= [`DIFFICULTY_BLOCKS_COUNT`]
    timestamps: Vec<u64>,
    /// The work done in the [`DIFFICULTY_ACCOUNTED_WINDOW_LEN`] window, this is an optimisation
    /// so we don't need to keep track of cumulative difficulties as well as timestamps.
    windowed_work: u128,
    /// The last height we accounted for.
    last_accounted_height: u64,
}

impl DifficultyCalculator {
    pub async fn init<D: Database + Clone>(mut database: D) -> Result<Self, Error> {
        let DatabaseResponse::ChainHeight(chain_height) = database
            .ready()
            .await?
            .call(DatabaseRequest::ChainHeight)
            .await?
        else {
            panic!("Database sent incorrect response")
        };

        DifficultyCalculator::init_from_chain_height(chain_height, database).await
    }

    pub async fn init_from_chain_height<D: Database + Clone>(
        chain_height: u64,
        mut database: D,
    ) -> Result<Self, Error> {
        let block_start = chain_height.saturating_sub(DIFFICULTY_BLOCKS_COUNT);

        let timestamps =
            get_blocks_in_range_timestamps(database.clone(), block_start..chain_height).await?;

        tracing::debug!(
            "Current chain height: {}, accounting for {} blocks timestamps",
            chain_height,
            timestamps.len()
        );

        let mut diff = DifficultyCalculator {
            timestamps,
            windowed_work: 0,
            last_accounted_height: chain_height - 1,
        };

        diff.update_windowed_work(&mut database).await?;

        Ok(diff)
    }

    pub async fn resync<D: Database + Clone>(&mut self, mut database: D) -> Result<(), Error> {
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

    async fn update_windowed_work<D: Database>(&mut self, mut database: D) -> Result<(), Error> {
        let block_start = (self.last_accounted_height + 1).saturating_sub(DIFFICULTY_BLOCKS_COUNT);

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

    pub fn next_difficulty(&self, hf: &HardFork) -> u128 {
        if self.timestamps.len() <= 1 {
            return 1;
        }

        let mut sorted_timestamps = self.timestamps.clone();
        sorted_timestamps.drain(DIFFICULTY_WINDOW..);
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

#[instrument(skip(database))]
async fn get_blocks_in_range_timestamps<D: Database + Clone>(
    database: D,
    block_heights: Range<u64>,
) -> Result<Vec<u64>, Error> {
    let start = block_heights.start;
    let mut timestamps = Vec::with_capacity(
        TryInto::<usize>::try_into(block_heights.end - start)
            .expect("Height does not fit into usize!"),
    );

    let mut timestamp_fut = FuturesOrdered::from_iter(block_heights.map(|height| {
        get_block_timestamp(database.clone(), height).map_ok(move |res| (height, res))
    }));

    while let Some(res) = timestamp_fut.next().await {
        let (height, timestamp): (u64, u64) = res?;
        tracing::debug!("Block timestamp for height: {} = {:?}", height, timestamp);

        timestamps.push(timestamp);
    }

    Ok(timestamps)
}

async fn get_block_timestamp<D: Database>(database: D, height: u64) -> Result<u64, Error> {
    let DatabaseResponse::BlockPOWInfo(pow) = database
        .oneshot(DatabaseRequest::BlockPOWInfo(height.into()))
        .await?
    else {
        panic!("Database service sent incorrect response!");
    };
    Ok(pow.timestamp)
}

async fn get_block_cum_diff<D: Database>(database: D, height: u64) -> Result<u128, Error> {
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

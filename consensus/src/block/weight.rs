use std::cmp::{max, min};
use std::ops::Range;

use monero_serai::{block::Block, transaction::Transaction};
use tower::ServiceExt;
use tracing::instrument;

use crate::{hardforks::HardFork, Database, DatabaseRequest, DatabaseResponse, Error};

const PENALTY_FREE_ZONE_1: usize = 20000;
const PENALTY_FREE_ZONE_2: usize = 60000;
const PENALTY_FREE_ZONE_5: usize = 300000;

const SHORT_TERM_WINDOW: u64 = 100;
const LONG_TERM_WINDOW: u64 = 100000;

#[derive(Debug)]
pub struct BlockWeightInfo {
    pub block_weight: usize,
    pub long_term_weight: usize,
}

/// Calculates the blocks weight.
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks/weight_limit.html#blocks-weight
pub fn block_weight(block: &Block, txs: &[Transaction]) -> usize {
    txs.iter()
        .chain([&block.miner_tx])
        .map(|tx| tx.weight())
        .sum()
}

/// Returns the penalty free zone
///
/// https://cuprate.github.io/monero-book/consensus_rules/blocks/weight_limit.html#penalty-free-zone
pub fn penalty_free_zone(hf: &HardFork) -> usize {
    if hf == &HardFork::V1 {
        PENALTY_FREE_ZONE_1
    } else if hf.in_range(&HardFork::V2, &HardFork::V5) {
        PENALTY_FREE_ZONE_2
    } else {
        PENALTY_FREE_ZONE_5
    }
}

pub struct BlockWeightsCache {
    /// This list is not sorted.
    short_term_block_weights: Vec<usize>,
    /// This list is sorted.
    long_term_weights: Vec<usize>,
    /// The height of the top block.
    tip_height: u64,
}

impl BlockWeightsCache {
    pub async fn init<D: Database + Clone>(mut database: D) -> Result<Self, Error> {
        let DatabaseResponse::ChainHeight(chain_height) = database
            .ready()
            .await?
            .call(DatabaseRequest::ChainHeight)
            .await?
        else {
            panic!("Database sent incorrect response!");
        };

        Self::init_from_chain_height(chain_height, database).await
    }

    pub async fn init_from_chain_height<D: Database + Clone>(
        chain_height: u64,
        database: D,
    ) -> Result<Self, Error> {
        let mut long_term_weights = get_long_term_weight_in_range(
            chain_height.saturating_sub(LONG_TERM_WINDOW)..chain_height,
            database.clone(),
        )
        .await?;

        long_term_weights.sort_unstable();
        tracing::debug!(
            "Sorted long term weights with length: {}",
            long_term_weights.len()
        );

        let short_term_block_weights = get_blocks_weight_in_range(
            chain_height.saturating_sub(SHORT_TERM_WINDOW)..chain_height,
            database,
        )
        .await?;

        Ok(BlockWeightsCache {
            short_term_block_weights,
            long_term_weights,
            tip_height: chain_height - 1,
        })
    }

    pub fn next_block_long_term_weight(&self, hf: &HardFork, block_weight: usize) -> usize {
        calculate_block_long_term_weight(hf, block_weight, &self.long_term_weights)
    }
}

pub fn calculate_block_long_term_weight(
    hf: &HardFork,
    block_weight: usize,
    sorted_long_term_window: &[usize],
) -> usize {
    if hf.in_range(&HardFork::V1, &HardFork::V10) {
        return block_weight;
    }

    let long_term_median = max(penalty_free_zone(hf), median(sorted_long_term_window));

    let (short_term_constraint, adjusted_block_weight) =
        if hf.in_range(&HardFork::V10, &HardFork::V15) {
            let stc = long_term_median + long_term_median * 2 / 5;
            (stc, block_weight)
        } else {
            let stc = long_term_median + long_term_median * 7 / 10;
            (stc, max(block_weight, long_term_median * 10 / 17))
        };

    min(short_term_constraint, adjusted_block_weight)
}

fn get_mid(a: usize, b: usize) -> usize {
    // https://github.com/monero-project/monero/blob/90294f09ae34ef96f3dea5fea544816786df87c8/contrib/epee/include/misc_language.h#L43
    (a / 2) + (b / 2) + ((a - 2 * (a / 2)) + (b - 2 * (b / 2))) / 2
}

fn median(array: &[usize]) -> usize {
    let mid = array.len() / 2;

    if array.len() == 1 {
        return array[0];
    }

    if array.len() % 2 == 0 {
        get_mid(array[mid - 1], array[mid])
    } else {
        array[mid]
    }
}

#[instrument(skip(database))]
async fn get_blocks_weight_in_range<D: Database + Clone>(
    range: Range<u64>,
    database: D,
) -> Result<Vec<usize>, Error> {
    let DatabaseResponse::BlockWeightsInRange(weights) = database
        .oneshot(DatabaseRequest::BlockWeightsInRange(range))
        .await?
    else {
        panic!()
    };

    Ok(weights.into_iter().map(|info| info.block_weight).collect())
}

#[instrument(skip(database))]
async fn get_long_term_weight_in_range<D: Database + Clone>(
    range: Range<u64>,
    database: D,
) -> Result<Vec<usize>, Error> {
    let DatabaseResponse::BlockWeightsInRange(weights) = database
        .oneshot(DatabaseRequest::BlockWeightsInRange(range))
        .await?
    else {
        panic!()
    };

    Ok(weights
        .into_iter()
        .map(|info| info.long_term_weight)
        .collect())
}

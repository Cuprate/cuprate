//! # Block Weights
//!
//! This module contains calculations for block weights, including calculating block weight
//! limits, effective medians and long term block weights.
//!
//! For more information please see the [block weights chapter](https://cuprate.github.io/monero-book/consensus_rules/blocks/weight_limit.html)
//! in the Monero Book.
//!
use std::{
    cmp::{max, min},
    ops::Range,
};

use tower::ServiceExt;
use tracing::instrument;

use cuprate_consensus_rules::blocks::{penalty_free_zone, PENALTY_FREE_ZONE_5};
use cuprate_helper::{asynch::rayon_spawn_async, num::RollingMedian};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    Chain,
};

use crate::{ContextCacheError, Database, HardFork};

/// The short term block weight window.
pub const SHORT_TERM_WINDOW: usize = 100;
/// The long term block weight window.
pub const LONG_TERM_WINDOW: usize = 100000;

/// Configuration for the block weight cache.
///
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct BlockWeightsCacheConfig {
    short_term_window: usize,
    long_term_window: usize,
}

impl BlockWeightsCacheConfig {
    /// Creates a new [`BlockWeightsCacheConfig`]
    pub const fn new(short_term_window: usize, long_term_window: usize) -> Self {
        Self {
            short_term_window,
            long_term_window,
        }
    }

    /// Returns the [`BlockWeightsCacheConfig`] for all networks (They are all the same as mainnet).
    pub const fn main_net() -> Self {
        Self {
            short_term_window: SHORT_TERM_WINDOW,
            long_term_window: LONG_TERM_WINDOW,
        }
    }
}

/// A cache used to calculate block weight limits, the effective median and
/// long term block weights.
///
/// These calculations require a lot of data from the database so by caching
/// this data it reduces the load on the database.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BlockWeightsCache {
    /// The short term block weights.
    short_term_block_weights: RollingMedian<usize>,
    /// The long term block weights.
    long_term_weights: RollingMedian<usize>,

    /// The height of the top block.
    pub(crate) tip_height: usize,

    pub(crate) config: BlockWeightsCacheConfig,
}

impl BlockWeightsCache {
    /// Initialize the [`BlockWeightsCache`] at the the given chain height.
    #[instrument(name = "init_weight_cache", level = "info", skip(database, config))]
    pub async fn init_from_chain_height<D: Database + Clone>(
        chain_height: usize,
        config: BlockWeightsCacheConfig,
        database: D,
        chain: Chain,
    ) -> Result<Self, ContextCacheError> {
        tracing::info!("Initializing weight cache this may take a while.");

        let long_term_weights = get_long_term_weight_in_range(
            chain_height.saturating_sub(config.long_term_window)..chain_height,
            database.clone(),
            chain,
        )
        .await?;

        let short_term_block_weights = get_blocks_weight_in_range(
            chain_height.saturating_sub(config.short_term_window)..chain_height,
            database,
            chain,
        )
        .await?;

        tracing::info!("Initialized block weight cache, chain-height: {:?}, long term weights length: {:?}, short term weights length: {:?}", chain_height, long_term_weights.len(), short_term_block_weights.len());

        Ok(Self {
            short_term_block_weights: rayon_spawn_async(move || {
                RollingMedian::from_vec(short_term_block_weights, config.short_term_window)
            })
            .await,
            long_term_weights: rayon_spawn_async(move || {
                RollingMedian::from_vec(long_term_weights, config.long_term_window)
            })
            .await,
            tip_height: chain_height - 1,
            config,
        })
    }

    /// Pop some blocks from the top of the cache.
    ///
    /// The cache will be returned to the state it would have been in `numb_blocks` ago.
    #[instrument(name = "pop_blocks_weight_cache", skip_all, fields(numb_blocks = numb_blocks))]
    pub async fn pop_blocks_main_chain<D: Database + Clone>(
        &mut self,
        numb_blocks: usize,
        database: D,
    ) -> Result<(), ContextCacheError> {
        if self.long_term_weights.window_len() <= numb_blocks {
            // More blocks to pop than we have in the cache, so just restart a new cache.
            *self = Self::init_from_chain_height(
                self.tip_height - numb_blocks + 1,
                self.config,
                database,
                Chain::Main,
            )
            .await?;

            return Ok(());
        }

        let chain_height = self.tip_height + 1;

        let old_long_term_weights = if let Some(new_long_term_start_height) = chain_height
            .checked_sub(self.config.long_term_window + numb_blocks)
        {
            get_long_term_weight_in_range(
                new_long_term_start_height
                    // current_chain_height - self.long_term_weights.len() blocks are already in the cache.
                    ..(new_long_term_start_height + numb_blocks),
                database.clone(),
                Chain::Main,
            )
                .await?
        } else {
            vec![]
        };

        let old_short_term_weights =if let Some(new_short_term_start_height) = chain_height
            .checked_sub(self.config.short_term_window + numb_blocks) {
             get_blocks_weight_in_range(
                new_short_term_start_height
                    // current_chain_height - self.long_term_weights.len() blocks are already in the cache.
                    ..(min(numb_blocks, self.short_term_block_weights.window_len()) + new_short_term_start_height),
                database,
                Chain::Main,
            )
                .await?
        } else {
            vec![]
        };

        for _ in 0..numb_blocks {
            self.short_term_block_weights.pop_back();
            self.long_term_weights.pop_back();
        }

        self.long_term_weights.append_front(old_long_term_weights);
        self.short_term_block_weights
            .append_front(old_short_term_weights);
        self.tip_height -= numb_blocks;

        Ok(())
    }

    /// Add a new block to the cache.
    ///
    /// The `block_height` **MUST** be one more than the last height the cache has
    /// seen.
    pub fn new_block(&mut self, block_height: usize, block_weight: usize, long_term_weight: usize) {
        assert_eq!(self.tip_height + 1, block_height);
        self.tip_height += 1;
        tracing::debug!(
            "Adding new block's {} weights to block cache, weight: {}, long term weight: {}",
            self.tip_height,
            block_weight,
            long_term_weight
        );

        self.long_term_weights.push(long_term_weight);

        self.short_term_block_weights.push(block_weight);
    }

    /// Returns the median long term weight over the last [`LONG_TERM_WINDOW`] blocks, or custom amount of blocks in the config.
    pub fn median_long_term_weight(&self) -> usize {
        self.long_term_weights.median()
    }

    /// Returns the median weight over the last [`SHORT_TERM_WINDOW`] blocks, or custom amount of blocks in the config.
    pub fn median_short_term_weight(&self) -> usize {
        self.short_term_block_weights.median()
    }

    /// Returns the effective median weight, used for block reward calculations and to calculate
    /// the block weight limit.
    ///
    /// See: <https://cuprate.github.io/monero-book/consensus_rules/blocks/weight_limit.html#calculating-effective-median-weight>
    pub fn effective_median_block_weight(&self, hf: HardFork) -> usize {
        calculate_effective_median_block_weight(
            hf,
            self.median_short_term_weight(),
            self.median_long_term_weight(),
        )
    }

    /// Returns the median weight used to calculate block reward punishment.
    ///
    /// <https://cuprate.github.io/monero-book/consensus_rules/blocks/reward.html#calculating-block-reward>
    pub fn median_for_block_reward(&self, hf: HardFork) -> usize {
        if hf < HardFork::V12 {
            self.median_short_term_weight()
        } else {
            self.effective_median_block_weight(hf)
        }
        .max(penalty_free_zone(hf))
    }
}

/// Calculates the effective median with the long term and short term median.
fn calculate_effective_median_block_weight(
    hf: HardFork,
    median_short_term_weight: usize,
    median_long_term_weight: usize,
) -> usize {
    if hf < HardFork::V10 {
        return median_short_term_weight.max(penalty_free_zone(hf));
    }

    let long_term_median = median_long_term_weight.max(PENALTY_FREE_ZONE_5);
    let short_term_median = median_short_term_weight;
    let effective_median = if hf >= HardFork::V10 && hf < HardFork::V15 {
        min(
            max(PENALTY_FREE_ZONE_5, short_term_median),
            50 * long_term_median,
        )
    } else {
        min(
            max(long_term_median, short_term_median),
            50 * long_term_median,
        )
    };

    effective_median.max(penalty_free_zone(hf))
}

/// Calculates a blocks long term weight.
pub fn calculate_block_long_term_weight(
    hf: HardFork,
    block_weight: usize,
    long_term_median: usize,
) -> usize {
    if hf < HardFork::V10 {
        return block_weight;
    }

    let long_term_median = max(penalty_free_zone(hf), long_term_median);

    let (short_term_constraint, adjusted_block_weight) =
        if hf >= HardFork::V10 && hf < HardFork::V15 {
            let stc = long_term_median + long_term_median * 2 / 5;
            (stc, block_weight)
        } else {
            let stc = long_term_median + long_term_median * 7 / 10;
            (stc, max(block_weight, long_term_median * 10 / 17))
        };

    min(short_term_constraint, adjusted_block_weight)
}

/// Gets the block weights from the blocks with heights in the range provided.
#[instrument(name = "get_block_weights", skip(database))]
async fn get_blocks_weight_in_range<D: Database + Clone>(
    range: Range<usize>,
    database: D,
    chain: Chain,
) -> Result<Vec<usize>, ContextCacheError> {
    tracing::info!("getting block weights.");

    let BlockchainResponse::BlockExtendedHeaderInRange(ext_headers) = database
        .oneshot(BlockchainReadRequest::BlockExtendedHeaderInRange(
            range, chain,
        ))
        .await?
    else {
        panic!("Database sent incorrect response!")
    };

    Ok(ext_headers
        .into_iter()
        .map(|info| info.block_weight)
        .collect())
}

/// Gets the block long term weights from the blocks with heights in the range provided.
#[instrument(name = "get_long_term_weights", skip(database), level = "info")]
async fn get_long_term_weight_in_range<D: Database + Clone>(
    range: Range<usize>,
    database: D,
    chain: Chain,
) -> Result<Vec<usize>, ContextCacheError> {
    tracing::info!("getting block long term weights.");

    let BlockchainResponse::BlockExtendedHeaderInRange(ext_headers) = database
        .oneshot(BlockchainReadRequest::BlockExtendedHeaderInRange(
            range, chain,
        ))
        .await?
    else {
        panic!("Database sent incorrect response!")
    };

    Ok(ext_headers
        .into_iter()
        .map(|info| info.long_term_weight)
        .collect())
}

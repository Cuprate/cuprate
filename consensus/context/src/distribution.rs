//! Output Distribution Module
//!
//! This module handles keeping track of the data required to serve the output distribution.
//! This data is currently the cumulative number of RCT outputs in each block.
//!
use std::{num::NonZero, ops::Range};

use tower::ServiceExt;
use tracing::instrument;

use cuprate_helper::cast::{u64_to_usize, usize_to_u64};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    rpc::OutputDistributionData,
    OutputAmount, OutputDistributionInput, VerifiedBlockInformation,
};

use crate::{hardforks::HardForkConfig, ContextCacheError, Database, HardFork};

/// A cache of the cumulative RCT output count per block.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CumulativeRctOutsCache {
    /// The HF v4 activation height.
    pub start_height: usize,
    /// The cumulative RCT output count for blocks `start_height..chain_height`.
    pub cumulative_rct_outs: Vec<u64>,
}

impl CumulativeRctOutsCache {
    /// Initialize the RCT outs cache from the specified chain height.
    #[instrument(
        name = "init_rct_outs_cache",
        level = "info",
        skip(hard_fork_cfg, database)
    )]
    pub async fn init_from_chain_height<D: Database + Clone>(
        chain_height: usize,
        hard_fork_cfg: HardForkConfig,
        database: D,
    ) -> Result<Self, ContextCacheError> {
        // The first height an RCT output can appear at is the HF v4 activation height.
        let rct_start_height = hard_fork_cfg.info.info_for_hf(&HardFork::V4).height();

        let cumulative_rct_outs = if rct_start_height < chain_height {
            get_cumulative_rct_outs(database, rct_start_height..chain_height).await?
        } else {
            Vec::new()
        };

        Ok(Self {
            start_height: rct_start_height,
            cumulative_rct_outs,
        })
    }

    /// Add a new block to the RCT outs cache.
    pub fn new_block(&mut self, height: usize, numb_rct_outputs: usize) {
        if height < self.start_height {
            debug_assert_eq!(numb_rct_outputs, 0);
            return;
        }
        assert_eq!(self.start_height + self.cumulative_rct_outs.len(), height);

        let last = self.cumulative_rct_outs.last().copied().unwrap_or(0);
        self.cumulative_rct_outs
            .push(last + usize_to_u64(numb_rct_outputs));
    }

    /// Pop some blocks from the top of the cache.
    pub fn pop_blocks_main_chain(&mut self, numb_blocks: usize) {
        self.cumulative_rct_outs
            .truncate(self.cumulative_rct_outs.len().saturating_sub(numb_blocks));
    }

    /// Returns the output distribution for the request.
    ///
    /// The RCT distribution is served from the cache, pre-RCT amounts
    /// are forwarded to the database.
    pub async fn distribution<D: Database>(
        &self,
        input: OutputDistributionInput,
        chain_height: usize,
        database: D,
    ) -> Result<Vec<OutputDistributionData>, tower::BoxError> {
        if input.to_height.is_some_and(|h| h.get() < input.from_height) {
            return Err("`to_height` is below `from_height`".into());
        }

        let to_height = input.to_height.map_or(chain_height - 1, |h| {
            let h = h.get();
            u64_to_usize(h)
        });

        if to_height >= chain_height {
            return Err("`to_height` is above the chain height".into());
        }

        let mut pre_rct = get_pre_rct_output_distribution(database, &input).await?;

        Ok(input
            .amounts
            .iter()
            .map(|amount| match amount {
                OutputAmount::Rct => self.rct_distribution(
                    u64_to_usize(input.from_height),
                    to_height,
                    input.cumulative,
                ),
                OutputAmount::PreRct(_) => {
                    pre_rct.next().expect("one distribution per pre-RCT amount")
                }
            })
            .collect())
    }

    /// Returns the RCT output distribution for blocks `from_height..=to_height`.
    ///
    /// # Invariant
    ///
    /// `to_height` must be below the chain height.
    pub fn rct_distribution(
        &self,
        from_height: usize,
        to_height: usize,
        cumulative: bool,
    ) -> OutputDistributionData {
        // clamp the start to the start of RCT, like monerod.
        let start_height = from_height.max(self.start_height);

        if start_height > to_height {
            return OutputDistributionData {
                amount: 0,
                distribution: Vec::new(),
                start_height: usize_to_u64(start_height),
                base: 0,
            };
        }

        let idx = |height: usize| height - self.start_height;

        // The value one block below the range, the base to calculate real values from
        // cumulative ones.
        let base = if start_height <= self.start_height {
            0
        } else {
            self.cumulative_rct_outs[idx(start_height - 1)]
        };

        let mut distribution =
            self.cumulative_rct_outs[idx(start_height)..=idx(to_height)].to_vec();

        if !cumulative {
            let mut prev = base;
            for cumulative_outs in &mut distribution {
                let delta = *cumulative_outs - prev;
                prev = *cumulative_outs;
                *cumulative_outs = delta;
            }
        }

        OutputDistributionData {
            amount: 0,
            distribution,
            start_height: usize_to_u64(start_height),
            base,
        }
    }
}

/// The number of RCT outputs in a block.
pub fn rct_output_count(block: &VerifiedBlockInformation) -> usize {
    let miner_tx = block.block.miner_transaction();
    let miner_tx_outputs = if miner_tx.version() == 2 {
        miner_tx.prefix().outputs.len()
    } else {
        0
    };

    miner_tx_outputs
        + block
            .txs
            .iter()
            .filter(|tx| tx.tx.version() == 2)
            .map(|tx| tx.tx.prefix().outputs.len())
            .sum::<usize>()
}

/// Returns the cumulative RCT output count for the main-chain blocks with heights in the specified range.
#[instrument(name = "get_cumulative_rct_outs", skip(database), level = "info")]
async fn get_cumulative_rct_outs<D: Database>(
    database: D,
    block_heights: Range<usize>,
) -> Result<Vec<u64>, ContextCacheError> {
    let BlockchainResponse::CumulativeRctOutsInRange(cumulative_rct_outs) = database
        .oneshot(BlockchainReadRequest::CumulativeRctOutsInRange(
            block_heights,
        ))
        .await?
    else {
        panic!("Database sent incorrect response");
    };

    Ok(cumulative_rct_outs)
}

/// Extracts the pre-RCT amounts from `input` and forwards them to the database.
async fn get_pre_rct_output_distribution<D: Database>(
    database: D,
    input: &OutputDistributionInput,
) -> Result<impl Iterator<Item = OutputDistributionData>, tower::BoxError> {
    let amounts: Vec<NonZero<u64>> = input
        .amounts
        .iter()
        .filter_map(|amount| match amount {
            OutputAmount::Rct => None,
            OutputAmount::PreRct(amount) => Some(*amount),
        })
        .collect();

    if amounts.is_empty() {
        return Ok(Vec::new().into_iter());
    }

    let input = OutputDistributionInput {
        amounts,
        cumulative: input.cumulative,
        from_height: input.from_height,
        to_height: input.to_height,
    };

    let BlockchainResponse::PreRctOutputDistribution(distributions) = database
        .oneshot(BlockchainReadRequest::PreRctOutputDistribution(input))
        .await?
    else {
        panic!("Database sent incorrect response");
    };

    Ok(distributions.into_iter())
}

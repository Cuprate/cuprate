//! # Blockchain Context
//!
//! This module contains a service to get cached context from the blockchain: [`BlockChainContext`].
//! This is used during contextual validation, this does not have all the data for contextual validation
//! (outputs) for that you will need a [`Database`].
//!
use std::{
    cmp::min,
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{channel::oneshot, FutureExt};
use tokio::sync::mpsc;
use tokio_util::sync::PollSender;
use tower::Service;

use cuprate_consensus_rules::{blocks::ContextToVerifyBlock, current_unix_timestamp, HardFork};

use crate::{Database, ExtendedConsensusError};

pub(crate) mod difficulty;
pub(crate) mod hardforks;
pub(crate) mod rx_vms;
pub(crate) mod weight;

mod alt_chains;
mod task;
mod tokens;

use cuprate_types::{Chain, ChainInfo, FeeEstimate, HardForkInfo};
use difficulty::DifficultyCache;
use rx_vms::RandomXVm;
use weight::BlockWeightsCache;

pub(crate) use alt_chains::{sealed::AltChainRequestToken, AltChainContextCache};
pub use difficulty::DifficultyCacheConfig;
pub use hardforks::HardForkConfig;
pub use tokens::*;
pub use weight::BlockWeightsCacheConfig;

pub(crate) const BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW: u64 = 60;

/// Config for the context service.
pub struct ContextConfig {
    /// Hard-forks config.
    pub hard_fork_cfg: HardForkConfig,
    /// Difficulty config.
    pub difficulty_cfg: DifficultyCacheConfig,
    /// Block weight config.
    pub weights_config: BlockWeightsCacheConfig,
}

impl ContextConfig {
    /// Get the config for main-net.
    pub const fn main_net() -> Self {
        Self {
            hard_fork_cfg: HardForkConfig::main_net(),
            difficulty_cfg: DifficultyCacheConfig::main_net(),
            weights_config: BlockWeightsCacheConfig::main_net(),
        }
    }

    /// Get the config for stage-net.
    pub const fn stage_net() -> Self {
        Self {
            hard_fork_cfg: HardForkConfig::stage_net(),
            // These 2 have the same config as main-net.
            difficulty_cfg: DifficultyCacheConfig::main_net(),
            weights_config: BlockWeightsCacheConfig::main_net(),
        }
    }

    /// Get the config for test-net.
    pub const fn test_net() -> Self {
        Self {
            hard_fork_cfg: HardForkConfig::test_net(),
            // These 2 have the same config as main-net.
            difficulty_cfg: DifficultyCacheConfig::main_net(),
            weights_config: BlockWeightsCacheConfig::main_net(),
        }
    }
}

/// Initialize the blockchain context service.
///
/// This function will request a lot of data from the database so it may take a while.
pub async fn initialize_blockchain_context<D>(
    cfg: ContextConfig,
    database: D,
) -> Result<BlockChainContextService, ExtendedConsensusError>
where
    D: Database + Clone + Send + Sync + 'static,
    D::Future: Send + 'static,
{
    let context_task = task::ContextTask::init_context(cfg, database).await?;

    // TODO: make buffer size configurable.
    let (tx, rx) = mpsc::channel(15);

    tokio::spawn(context_task.run(rx));

    Ok(BlockChainContextService {
        channel: PollSender::new(tx),
    })
}

/// Raw blockchain context, gotten from [`BlockChainContext`]. This data may turn invalid so is not ok to keep
/// around. You should keep around [`BlockChainContext`] instead.
#[derive(Debug, Clone)]
pub struct RawBlockChainContext {
    /// The current cumulative difficulty.
    pub cumulative_difficulty: u128,
    /// Context to verify a block, as needed by [`cuprate-consensus-rules`]
    pub context_to_verify_block: ContextToVerifyBlock,
    /// The median long term block weight.
    median_long_term_weight: usize,
    /// The top blocks timestamp (will be [`None`] if the top block is the genesis).
    top_block_timestamp: Option<u64>,
}

impl std::ops::Deref for RawBlockChainContext {
    type Target = ContextToVerifyBlock;
    fn deref(&self) -> &Self::Target {
        &self.context_to_verify_block
    }
}

impl RawBlockChainContext {
    /// Returns the timestamp the should be used when checking locked outputs.
    ///
    /// ref: <https://cuprate.github.io/monero-book/consensus_rules/transactions/unlock_time.html#getting-the-current-time>
    pub fn current_adjusted_timestamp_for_time_lock(&self) -> u64 {
        if self.current_hf < HardFork::V13 || self.median_block_timestamp.is_none() {
            current_unix_timestamp()
        } else {
            // This is safe as we just checked if this was None.
            let median = self.median_block_timestamp.unwrap();

            let adjusted_median = median
                + (BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW + 1) * self.current_hf.block_time().as_secs()
                    / 2;

            // This is safe as we just checked if the median was None and this will only be none for genesis and the first block.
            let adjusted_top_block =
                self.top_block_timestamp.unwrap() + self.current_hf.block_time().as_secs();

            min(adjusted_median, adjusted_top_block)
        }
    }

    /// Returns the next blocks long term weight from its block weight.
    pub fn next_block_long_term_weight(&self, block_weight: usize) -> usize {
        weight::calculate_block_long_term_weight(
            self.current_hf,
            block_weight,
            self.median_long_term_weight,
        )
    }
}

/// Blockchain context which keeps a token of validity so users will know when the data is no longer valid.
#[derive(Debug, Clone)]
pub struct BlockChainContext {
    /// A token representing this data's validity.
    validity_token: ValidityToken,
    /// The actual block chain context.
    raw: RawBlockChainContext,
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("data is no longer valid")]
pub struct DataNoLongerValid;

impl BlockChainContext {
    /// Checks if the data is still valid.
    pub fn is_still_valid(&self) -> bool {
        self.validity_token.is_data_valid()
    }

    /// Checks if the data is valid returning an Err if not and a reference to the blockchain context if
    /// it is.
    pub fn blockchain_context(&self) -> Result<&RawBlockChainContext, DataNoLongerValid> {
        if !self.is_still_valid() {
            return Err(DataNoLongerValid);
        }
        Ok(&self.raw)
    }

    /// Returns the blockchain context without checking the validity token.
    pub const fn unchecked_blockchain_context(&self) -> &RawBlockChainContext {
        &self.raw
    }
}

/// Data needed from a new block to add it to the context cache.
#[derive(Debug, Clone)]
pub struct NewBlockData {
    /// The blocks hash.
    pub block_hash: [u8; 32],
    /// The blocks height.
    pub height: usize,
    /// The blocks timestamp.
    pub timestamp: u64,
    /// The blocks weight.
    pub weight: usize,
    /// long term weight of this block.
    pub long_term_weight: usize,
    /// The coins generated by this block.
    pub generated_coins: u64,
    /// The blocks hf vote.
    pub vote: HardFork,
    /// The cumulative difficulty of the chain.
    pub cumulative_difficulty: u128,
}

/// A request to the blockchain context cache.
#[derive(Debug, Clone)]
pub enum BlockChainContextRequest {
    /// Get the current blockchain context.
    Context,

    /// Gets the current `RandomX` VM.
    CurrentRxVm,

    /// Get the next difficulties for these blocks.
    ///
    /// Inputs: a list of block timestamps and hfs
    ///
    /// The number of difficulties returned will be one more than the number of timestamps/ hfs.
    BatchGetDifficulties(Vec<(u64, HardFork)>),

    /// Add a VM that has been created outside of the blockchain context service to the blockchain context.
    /// This is useful when batch calculating POW as you may need to create a new VM if you batch a lot of blocks together,
    /// it would be wasteful to then not give this VM to the context service to then use when it needs to init a VM with the same
    /// seed.
    ///
    /// This should include the seed used to init this VM and the VM.
    NewRXVM(([u8; 32], Arc<RandomXVm>)),

    /// A request to add a new block to the cache.
    Update(NewBlockData),

    /// Pop blocks from the cache to the specified height.
    PopBlocks {
        /// The number of blocks to pop from the top of the chain.
        ///
        /// # Panics
        ///
        /// This will panic if the number of blocks will pop the genesis block.
        numb_blocks: usize,
    },

    /// Clear the alt chain context caches.
    ClearAltCache,

    //----------------------------------------------------------------------------------------------------------- AltChainRequests
    /// A request for an alt chain context cache.
    ///
    /// This variant is private and is not callable from outside this crate, the block verifier service will
    /// handle getting the alt cache.
    AltChainContextCache {
        /// The previous block field in a [`BlockHeader`](monero_serai::block::BlockHeader).
        prev_id: [u8; 32],
        /// An internal token to prevent external crates calling this request.
        _token: AltChainRequestToken,
    },

    /// A request for a difficulty cache of an alternative chin.
    ///
    /// This variant is private and is not callable from outside this crate, the block verifier service will
    /// handle getting the difficulty cache of an alt chain.
    AltChainDifficultyCache {
        /// The previous block field in a [`BlockHeader`](monero_serai::block::BlockHeader).
        prev_id: [u8; 32],
        /// An internal token to prevent external crates calling this request.
        _token: AltChainRequestToken,
    },

    /// A request for a block weight cache of an alternative chin.
    ///
    /// This variant is private and is not callable from outside this crate, the block verifier service will
    /// handle getting the weight cache of an alt chain.
    AltChainWeightCache {
        /// The previous block field in a [`BlockHeader`](monero_serai::block::BlockHeader).
        prev_id: [u8; 32],
        /// An internal token to prevent external crates calling this request.
        _token: AltChainRequestToken,
    },

    /// A request for a RX VM for an alternative chin.
    ///
    /// Response variant: [`BlockChainContextResponse::AltChainRxVM`].
    ///
    /// This variant is private and is not callable from outside this crate, the block verifier service will
    /// handle getting the randomX VM of an alt chain.
    AltChainRxVM {
        /// The height the `RandomX` VM is needed for.
        height: usize,
        /// The chain to look in for the seed.
        chain: Chain,
        /// An internal token to prevent external crates calling this request.
        _token: AltChainRequestToken,
    },

    /// A request to add an alt chain context cache to the context cache.
    ///
    /// This variant is private and is not callable from outside this crate, the block verifier service will
    /// handle returning the alt cache to the context service.
    AddAltChainContextCache {
        /// The previous block field in a [`BlockHeader`](monero_serai::block::BlockHeader).
        prev_id: [u8; 32],
        /// The cache.
        cache: Box<AltChainContextCache>,
        /// An internal token to prevent external crates calling this request.
        _token: AltChainRequestToken,
    },

    /// Get information on a certain hardfork.
    HardForkInfo(HardFork),

    /// Get the current fee estimate.
    FeeEstimate {
        /// TODO
        grace_blocks: u64,
    },

    /// Get information on all the current alternate chains.
    AlternateChains,
}

pub enum BlockChainContextResponse {
    /// A generic Ok response.
    ///
    /// Response to:
    /// - [`BlockChainContextRequest::NewRXVM`]
    /// - [`BlockChainContextRequest::Update`]
    /// - [`BlockChainContextRequest::PopBlocks`]
    /// - [`BlockChainContextRequest::ClearAltCache`]
    /// - [`BlockChainContextRequest::AddAltChainContextCache`]
    Ok,

    /// Response to [`BlockChainContextRequest::Context`]
    Context(BlockChainContext),

    // TODO: why does this return a `HashMap` when the request is `CurrentRxVm`?
    /// Response to [`BlockChainContextRequest::CurrentRxVm`]
    ///
    /// A map of seed height to `RandomX` VMs.
    RxVms(HashMap<usize, Arc<RandomXVm>>),

    /// A list of difficulties.
    BatchDifficulties(Vec<u128>),

    /// An alt chain context cache.
    AltChainContextCache(Box<AltChainContextCache>),

    /// A difficulty cache for an alt chain.
    AltChainDifficultyCache(DifficultyCache),

    /// A randomX VM for an alt chain.
    AltChainRxVM(Arc<RandomXVm>),

    /// A weight cache for an alt chain
    AltChainWeightCache(BlockWeightsCache),

    /// Response to [`BlockChainContextRequest::HardForkInfo`]
    HardForkInfo(HardForkInfo),

    /// Response to [`BlockChainContextRequest::FeeEstimate`]
    FeeEstimate(FeeEstimate),

    /// Response to [`BlockChainContextRequest::AlternateChains`]
    ///
    /// If the inner [`Vec::is_empty`], there were no alternate chains.
    AlternateChains(Vec<ChainInfo>),
}

/// The blockchain context service.
#[derive(Clone)]
pub struct BlockChainContextService {
    channel: PollSender<task::ContextTaskRequest>,
}

impl Service<BlockChainContextRequest> for BlockChainContextService {
    type Response = BlockChainContextResponse;
    type Error = tower::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.channel
            .poll_reserve(cx)
            .map_err(|_| "Context service channel closed".into())
    }

    fn call(&mut self, req: BlockChainContextRequest) -> Self::Future {
        let (tx, rx) = oneshot::channel();

        let req = task::ContextTaskRequest {
            req,
            tx,
            span: tracing::Span::current(),
        };

        let res = self.channel.send_item(req);

        async move {
            res.map_err(|_| "Context service closed.")?;
            rx.await.expect("Oneshot closed without response!")
        }
        .boxed()
    }
}

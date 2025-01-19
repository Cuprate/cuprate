//! # Blockchain Context
//!
//! This crate contains a service to get cached context from the blockchain: [`BlockchainContext`].
//! This is used during contextual validation, this does not have all the data for contextual validation
//! (outputs) for that you will need a [`Database`].

// Used in documentation references for [`BlockChainContextRequest`]
// FIXME: should we pull in a dependency just to link docs?
use monero_serai as _;

use arc_swap::Cache;
use futures::{channel::oneshot, FutureExt};
use monero_serai::block::Block;
use std::{
    cmp::min,
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::mpsc;
use tokio_util::sync::PollSender;
use tower::Service;

use cuprate_consensus_rules::{
    blocks::ContextToVerifyBlock, current_unix_timestamp, ConsensusError, HardFork,
};

pub mod difficulty;
pub mod hardforks;
pub mod rx_vms;
pub mod weight;

mod alt_chains;
mod task;

use cuprate_types::{Chain, ChainInfo, FeeEstimate, HardForkInfo};
use difficulty::DifficultyCache;
use rx_vms::RandomXVm;
use weight::BlockWeightsCache;

pub use alt_chains::{sealed::AltChainRequestToken, AltChainContextCache};
pub use difficulty::DifficultyCacheConfig;
pub use hardforks::HardForkConfig;
pub use weight::BlockWeightsCacheConfig;

pub const BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW: u64 = 60;

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
) -> Result<BlockchainContextService, ContextCacheError>
where
    D: Database + Clone + Send + Sync + 'static,
    D::Future: Send + 'static,
{
    let (context_task, context_cache) = task::ContextTask::init_context(cfg, database).await?;

    // TODO: make buffer size configurable.
    let (tx, rx) = mpsc::channel(15);

    tokio::spawn(context_task.run(rx));

    Ok(BlockchainContextService {
        cached_context: Cache::new(context_cache),

        channel: PollSender::new(tx),
    })
}

/// Raw blockchain context, gotten from [`BlockchainContext`]. This data may turn invalid so is not ok to keep
/// around. You should keep around [`BlockchainContext`] instead.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BlockchainContext {
    /// The current cumulative difficulty.
    pub cumulative_difficulty: u128,
    /// Context to verify a block, as needed by [`cuprate-consensus-rules`]
    pub context_to_verify_block: ContextToVerifyBlock,
    /// The median long term block weight.
    median_long_term_weight: usize,
    /// The top blocks timestamp (will be [`None`] if the top block is the genesis).
    top_block_timestamp: Option<u64>,
}

impl std::ops::Deref for BlockchainContext {
    type Target = ContextToVerifyBlock;
    fn deref(&self) -> &Self::Target {
        &self.context_to_verify_block
    }
}

impl BlockchainContext {
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
    /// Gets all the current  `RandomX` VMs.
    CurrentRxVms,

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

    /// Get information on a certain hardfork.
    HardForkInfo(HardFork),

    /// Get the current fee estimate.
    FeeEstimate {
        /// TODO
        grace_blocks: u64,
    },

    /// Calculate proof-of-work for this block.
    CalculatePow {
        /// The hardfork of the protocol at this block height.
        hardfork: HardFork,
        /// The height of the block.
        height: usize,
        /// The block data.
        ///
        /// This is boxed because [`Block`] causes this enum to be 1200 bytes,
        /// where the 2nd variant is only 96 bytes.
        block: Box<Block>,
        /// The seed hash for the proof-of-work.
        seed_hash: [u8; 32],
    },

    /// Clear the alt chain context caches.
    ClearAltCache,

    /// Get information on all the current alternate chains.
    AltChains,

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

    /// Response to [`BlockChainContextRequest::CurrentRxVms`]
    ///
    /// A map of seed height to `RandomX` VMs.
    RxVms(HashMap<usize, Arc<RandomXVm>>),

    /// A list of difficulties.
    BatchDifficulties(Vec<u128>),

    /// Response to [`BlockChainContextRequest::HardForkInfo`]
    HardForkInfo(HardForkInfo),

    /// Response to [`BlockChainContextRequest::FeeEstimate`]
    FeeEstimate(FeeEstimate),

    /// Response to [`BlockChainContextRequest::CalculatePow`]
    CalculatePow([u8; 32]),

    /// Response to [`BlockChainContextRequest::AltChains`]
    ///
    /// If the inner [`Vec::is_empty`], there were no alternate chains.
    AltChains(Vec<ChainInfo>),

    /// An alt chain context cache.
    AltChainContextCache(Box<AltChainContextCache>),

    /// A difficulty cache for an alt chain.
    AltChainDifficultyCache(DifficultyCache),

    /// A randomX VM for an alt chain.
    AltChainRxVM(Arc<RandomXVm>),

    /// A weight cache for an alt chain
    AltChainWeightCache(BlockWeightsCache),
}

/// The blockchain context service.
#[derive(Clone)]
pub struct BlockchainContextService {
    cached_context: Cache<Arc<arc_swap::ArcSwap<BlockchainContext>>, Arc<BlockchainContext>>,

    channel: PollSender<task::ContextTaskRequest>,
}

impl BlockchainContextService {
    pub fn blockchain_context(&mut self) -> &BlockchainContext {
        self.cached_context.load()
    }
}

impl Service<BlockChainContextRequest> for BlockchainContextService {
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

#[derive(Debug, thiserror::Error)]
pub enum ContextCacheError {
    /// A consensus error.
    #[error("{0}")]
    ConErr(#[from] ConsensusError),
    /// A database error.
    #[error("Database error: {0}")]
    DBErr(#[from] tower::BoxError),
}

use __private::Database;

pub mod __private {
    use std::future::Future;

    use cuprate_types::blockchain::{BlockchainReadRequest, BlockchainResponse};

    /// A type alias trait used to represent a database, so we don't have to write [`tower::Service`] bounds
    /// everywhere.
    ///
    /// Automatically implemented for:
    /// ```ignore
    /// tower::Service<BCReadRequest, Response = BCResponse, Error = tower::BoxError>
    /// ```
    pub trait Database:
        tower::Service<
        BlockchainReadRequest,
        Response = BlockchainResponse,
        Error = tower::BoxError,
        Future = Self::Future2,
    >
    {
        type Future2: Future<Output = Result<Self::Response, Self::Error>> + Send + 'static;
    }

    impl<
            T: tower::Service<
                BlockchainReadRequest,
                Response = BlockchainResponse,
                Error = tower::BoxError,
            >,
        > Database for T
    where
        T::Future: Future<Output = Result<Self::Response, Self::Error>> + Send + 'static,
    {
        type Future2 = T::Future;
    }
}

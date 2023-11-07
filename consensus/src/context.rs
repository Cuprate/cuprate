//! # Blockchain Context
//!
//! This module contains a service to get cached context from the blockchain: [`BlockChainContext`].
//! This is used during contextual validation, this does not have all the data for contextual validation
//! (outputs) for that you will need a [`Database`].
//!

use std::{
    cmp::min,
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::FutureExt;
use tokio::sync::RwLock;
use tower::{Service, ServiceExt};

use crate::{helper::current_time, ConsensusError, Database, DatabaseRequest, DatabaseResponse};

mod difficulty;
mod hardforks;
mod weight;

#[cfg(test)]
mod tests;
mod tokens;

pub use difficulty::DifficultyCacheConfig;
pub use hardforks::{HardFork, HardForkConfig};
pub use tokens::*;
pub use weight::BlockWeightsCacheConfig;

const BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW: u64 = 60;

pub struct ContextConfig {
    pub hard_fork_cfg: HardForkConfig,
    pub difficulty_cfg: DifficultyCacheConfig,
    pub weights_config: BlockWeightsCacheConfig,
}

impl ContextConfig {
    pub fn main_net() -> ContextConfig {
        ContextConfig {
            hard_fork_cfg: HardForkConfig::main_net(),
            difficulty_cfg: DifficultyCacheConfig::main_net(),
            weights_config: BlockWeightsCacheConfig::main_net(),
        }
    }
}

pub async fn initialize_blockchain_context<D>(
    cfg: ContextConfig,
    mut database: D,
) -> Result<
    (
        impl Service<
                BlockChainContextRequest,
                Response = BlockChainContext,
                Error = tower::BoxError,
                Future = impl Future<Output = Result<BlockChainContext, tower::BoxError>>
                             + Send
                             + 'static,
            > + Clone
            + Send
            + Sync
            + 'static,
        impl Service<UpdateBlockchainCacheRequest, Response = (), Error = tower::BoxError>,
    ),
    ConsensusError,
>
where
    D: Database + Clone + Send + Sync + 'static,
    D::Future: Send + 'static,
{
    let ContextConfig {
        difficulty_cfg,
        weights_config,
        hard_fork_cfg,
    } = cfg;

    tracing::debug!("Initialising blockchain context");

    let DatabaseResponse::ChainHeight(chain_height, top_block_hash) = database
        .ready()
        .await?
        .call(DatabaseRequest::ChainHeight)
        .await?
    else {
        panic!("Database sent incorrect response!");
    };

    let DatabaseResponse::GeneratedCoins(already_generated_coins) = database
        .ready()
        .await?
        .call(DatabaseRequest::GeneratedCoins)
        .await?
    else {
        panic!("Database sent incorrect response!");
    };

    let db = database.clone();
    let difficulty_cache_handle = tokio::spawn(async move {
        difficulty::DifficultyCache::init_from_chain_height(chain_height, difficulty_cfg, db).await
    });

    let db = database.clone();
    let weight_cache_handle = tokio::spawn(async move {
        weight::BlockWeightsCache::init_from_chain_height(chain_height, weights_config, db).await
    });

    let db = database.clone();
    let hardfork_state_handle = tokio::spawn(async move {
        hardforks::HardForkState::init_from_chain_height(chain_height, hard_fork_cfg, db).await
    });

    let context_svc = BlockChainContextService {
        internal_blockchain_context: Arc::new(
            InternalBlockChainContext {
                current_validity_token: ValidityToken::new(),
                current_reorg_token: ReOrgToken::new(),
                difficulty_cache: difficulty_cache_handle.await.unwrap()?,
                weight_cache: weight_cache_handle.await.unwrap()?,
                hardfork_state: hardfork_state_handle.await.unwrap()?,
                chain_height,
                already_generated_coins,
                top_block_hash,
            }
            .into(),
        ),
    };

    let context_svc_update = context_svc.clone();

    Ok((context_svc_update.clone(), context_svc_update))
}

/// Raw blockchain context, gotten from [`BlockChainContext`]. This data may turn invalid so is not ok to keep
/// around. You should keep around [`BlockChainContext`] instead.
#[derive(Debug, Clone)]
pub struct RawBlockChainContext {
    /// The next blocks difficulty.
    pub next_difficulty: u128,
    /// The current cumulative difficulty.
    pub cumulative_difficulty: u128,
    /// The current effective median block weight.
    pub effective_median_weight: usize,
    /// The median long term block weight.
    median_long_term_weight: usize,
    /// Median weight to use for block reward calculations.
    pub median_weight_for_block_reward: usize,
    /// The amount of coins minted already.
    pub already_generated_coins: u64,
    /// The median timestamp over the last [`BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW`] blocks, will be None if there aren't
    /// [`BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW`] blocks.
    pub median_block_timestamp: Option<u64>,
    top_block_timestamp: Option<u64>,
    /// The height of the chain.
    pub chain_height: u64,
    /// The top blocks hash
    pub top_hash: [u8; 32],
    /// The current hard fork.
    pub current_hard_fork: HardFork,
    /// A token which is used to signal if a reorg has happened since creating the token.
    pub re_org_token: ReOrgToken,
}

impl RawBlockChainContext {
    /// Returns the timestamp the should be used when checking locked outputs.
    ///
    /// https://cuprate.github.io/monero-book/consensus_rules/transactions/unlock_time.html#getting-the-current-time
    pub fn current_adjusted_timestamp_for_time_lock(&self) -> u64 {
        if self.current_hard_fork < HardFork::V13 || self.median_block_timestamp.is_none() {
            current_time()
        } else {
            // This is safe as we just checked if this was None.
            let median = self.median_block_timestamp.unwrap();

            let adjusted_median = median
                + (BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW + 1)
                    * self.current_hard_fork.block_time().as_secs()
                    / 2;

            // This is safe as we just checked if the median was None and this will only be none for genesis and the first block.
            let adjusted_top_block =
                self.top_block_timestamp.unwrap() + self.current_hard_fork.block_time().as_secs();

            min(adjusted_median, adjusted_top_block)
        }
    }

    pub fn block_blob_size_limit(&self) -> usize {
        self.effective_median_weight * 2 - 600
    }

    pub fn block_weight_limit(&self) -> usize {
        self.median_weight_for_block_reward * 2
    }

    pub fn next_block_long_term_weight(&self, block_weight: usize) -> usize {
        weight::calculate_block_long_term_weight(
            &self.current_hard_fork,
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
}

#[derive(Debug, Clone)]
pub struct BlockChainContextRequest;

#[derive(Clone)]
struct InternalBlockChainContext {
    /// A token used to invalidate previous contexts when a new
    /// block is added to the chain.
    current_validity_token: ValidityToken,
    /// A token which is used to signal a reorg has happened.
    current_reorg_token: ReOrgToken,

    difficulty_cache: difficulty::DifficultyCache,
    weight_cache: weight::BlockWeightsCache,
    hardfork_state: hardforks::HardForkState,

    chain_height: u64,
    top_block_hash: [u8; 32],
    already_generated_coins: u64,
}

#[derive(Clone)]
pub struct BlockChainContextService {
    internal_blockchain_context: Arc<RwLock<InternalBlockChainContext>>,
}

impl Service<BlockChainContextRequest> for BlockChainContextService {
    type Response = BlockChainContext;
    type Error = tower::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: BlockChainContextRequest) -> Self::Future {
        let internal_blockchain_context = self.internal_blockchain_context.clone();

        async move {
            let internal_blockchain_context_lock = internal_blockchain_context.read().await;

            let InternalBlockChainContext {
                current_validity_token,
                current_reorg_token,
                difficulty_cache,
                weight_cache,
                hardfork_state,
                chain_height,
                top_block_hash,
                already_generated_coins,
            } = internal_blockchain_context_lock.deref();

            let current_hf = hardfork_state.current_hardfork();

            Ok(BlockChainContext {
                validity_token: current_validity_token.clone(),
                raw: RawBlockChainContext {
                    next_difficulty: difficulty_cache.next_difficulty(&current_hf),
                    cumulative_difficulty: difficulty_cache.cumulative_difficulty(),
                    effective_median_weight: weight_cache
                        .effective_median_block_weight(&current_hf)
                        .await,
                    median_long_term_weight: weight_cache.median_long_term_weight().await,
                    median_weight_for_block_reward: weight_cache
                        .median_for_block_reward(&current_hf)
                        .await,
                    already_generated_coins: *already_generated_coins,
                    top_block_timestamp: difficulty_cache.top_block_timestamp(),
                    median_block_timestamp: difficulty_cache.median_timestamp(
                        usize::try_from(BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW).unwrap(),
                    ),
                    chain_height: *chain_height,
                    top_hash: *top_block_hash,
                    current_hard_fork: current_hf,
                    re_org_token: current_reorg_token.clone(),
                },
            })
        }
        .boxed()
    }
}

// TODO: join these services, there is no need for 2.
pub struct UpdateBlockchainCacheRequest {
    pub new_top_hash: [u8; 32],
    pub height: u64,
    pub timestamp: u64,
    pub weight: usize,
    pub long_term_weight: usize,
    pub generated_coins: u64,
    pub vote: HardFork,
    pub cumulative_difficulty: u128,
}

impl tower::Service<UpdateBlockchainCacheRequest> for BlockChainContextService {
    type Response = ();
    type Error = tower::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, new: UpdateBlockchainCacheRequest) -> Self::Future {
        let internal_blockchain_context = self.internal_blockchain_context.clone();

        async move {
            let mut internal_blockchain_context_lock = internal_blockchain_context.write().await;

            let InternalBlockChainContext {
                current_validity_token,
                current_reorg_token: _,
                difficulty_cache,
                weight_cache,
                hardfork_state,
                chain_height,
                top_block_hash,
                already_generated_coins,
            } = internal_blockchain_context_lock.deref_mut();

            // Cancel the validity token and replace it with a new one.
            std::mem::replace(current_validity_token, ValidityToken::new()).set_data_invalid();

            difficulty_cache.new_block(new.height, new.timestamp, new.cumulative_difficulty);

            weight_cache.new_block(new.height, new.weight, new.long_term_weight);

            hardfork_state.new_block(new.vote, new.height);

            *chain_height = new.height + 1;
            *top_block_hash = new.new_top_hash;
            *already_generated_coins = already_generated_coins.saturating_add(new.generated_coins);

            Ok(())
        }
        .boxed()
    }
}

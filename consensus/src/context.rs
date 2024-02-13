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
    ops::DerefMut,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{
    lock::{Mutex, OwnedMutexGuard, OwnedMutexLockFuture},
    FutureExt,
};
use tower::{Service, ServiceExt};

use cuprate_consensus_rules::{blocks::ContextToVerifyBlock, current_unix_timestamp, HardFork};

use crate::{Database, DatabaseRequest, DatabaseResponse, ExtendedConsensusError};

pub(crate) mod difficulty;
pub(crate) mod hardforks;
pub(crate) mod rx_vms;
pub(crate) mod weight;

mod tokens;

pub use difficulty::DifficultyCacheConfig;
pub use hardforks::HardForkConfig;
use rx_vms::RandomXVM;
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

    pub fn stage_net() -> ContextConfig {
        ContextConfig {
            hard_fork_cfg: HardForkConfig::stage_net(),
            difficulty_cfg: DifficultyCacheConfig::main_net(),
            weights_config: BlockWeightsCacheConfig::main_net(),
        }
    }

    pub fn test_net() -> ContextConfig {
        ContextConfig {
            hard_fork_cfg: HardForkConfig::test_net(),
            difficulty_cfg: DifficultyCacheConfig::main_net(),
            weights_config: BlockWeightsCacheConfig::main_net(),
        }
    }
}

pub async fn initialize_blockchain_context<D>(
    cfg: ContextConfig,
    mut database: D,
) -> Result<
    impl Service<
            BlockChainContextRequest,
            Response = BlockChainContextResponse,
            Error = tower::BoxError,
            Future = impl Future<Output = Result<BlockChainContextResponse, tower::BoxError>>
                         + Send
                         + 'static,
        > + Clone
        + Send
        + Sync
        + 'static,
    ExtendedConsensusError,
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
    let hardfork_state_handle = tokio::spawn(async move {
        hardforks::HardForkState::init_from_chain_height(chain_height, hard_fork_cfg, db).await
    });

    let db = database.clone();
    let difficulty_cache_handle = tokio::spawn(async move {
        difficulty::DifficultyCache::init_from_chain_height(chain_height, difficulty_cfg, db).await
    });

    let db = database.clone();
    let weight_cache_handle = tokio::spawn(async move {
        weight::BlockWeightsCache::init_from_chain_height(chain_height, weights_config, db).await
    });

    let hardfork_state = hardfork_state_handle.await.unwrap()?;
    let current_hf = hardfork_state.current_hardfork();

    let db = database.clone();
    let rx_seed_handle = tokio::spawn(async move {
        rx_vms::RandomXVMCache::init_from_chain_height(chain_height, &current_hf, db).await
    });

    let context_svc = BlockChainContextService {
        internal_blockchain_context: Arc::new(
            InternalBlockChainContext {
                current_validity_token: ValidityToken::new(),
                current_reorg_token: ReOrgToken::new(),
                difficulty_cache: difficulty_cache_handle.await.unwrap()?,
                weight_cache: weight_cache_handle.await.unwrap()?,
                rx_seed_cache: rx_seed_handle.await.unwrap()?,
                hardfork_state,
                chain_height,
                already_generated_coins,
                top_block_hash,
            }
            .into(),
        ),
        lock_state: MutexLockState::Locked,
    };

    Ok(context_svc)
}

/// Raw blockchain context, gotten from [`BlockChainContext`]. This data may turn invalid so is not ok to keep
/// around. You should keep around [`BlockChainContext`] instead.
#[derive(Debug, Clone)]
pub struct RawBlockChainContext {
    /// The current cumulative difficulty.
    pub cumulative_difficulty: u128,
    /// A token which is used to signal if a reorg has happened since creating the token.
    pub re_org_token: ReOrgToken,
    pub rx_vms: HashMap<u64, Arc<RandomXVM>>,
    pub context_to_verify_block: ContextToVerifyBlock,
    /// The median long term block weight.
    median_long_term_weight: usize,
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
    /// <https://cuprate.github.io/monero-book/consensus_rules/transactions/unlock_time.html#getting-the-current-time>
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

    pub fn block_blob_size_limit(&self) -> usize {
        self.effective_median_weight * 2 - 600
    }

    pub fn block_weight_limit(&self) -> usize {
        self.median_weight_for_block_reward * 2
    }

    pub fn next_block_long_term_weight(&self, block_weight: usize) -> usize {
        weight::calculate_block_long_term_weight(
            &self.current_hf,
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
    pub fn unchecked_blockchain_context(&self) -> &RawBlockChainContext {
        &self.raw
    }
}

#[derive(Debug, Clone)]
pub struct UpdateBlockchainCacheData {
    pub new_top_hash: [u8; 32],
    pub height: u64,
    pub timestamp: u64,
    pub weight: usize,
    pub long_term_weight: usize,
    pub generated_coins: u64,
    pub vote: HardFork,
    pub cumulative_difficulty: u128,
}

#[derive(Debug, Clone)]
pub enum BlockChainContextRequest {
    GetContext,
    /// Get the next difficulties for these blocks.
    ///
    /// Inputs: a list of block timestamps and hfs
    ///
    /// The number of difficulties returned will be one more than the number of timestamps/ hfs.
    BatchGetDifficulties(Vec<(u64, HardFork)>),
    NewRXVM(([u8; 32], Arc<RandomXVM>)),
    Update(UpdateBlockchainCacheData),
}

pub enum BlockChainContextResponse {
    Context(BlockChainContext),
    BatchDifficulties(Vec<u128>),
    Ok,
}
struct InternalBlockChainContext {
    /// A token used to invalidate previous contexts when a new
    /// block is added to the chain.
    current_validity_token: ValidityToken,
    /// A token which is used to signal a reorg has happened.
    current_reorg_token: ReOrgToken,

    difficulty_cache: difficulty::DifficultyCache,
    weight_cache: weight::BlockWeightsCache,
    rx_seed_cache: rx_vms::RandomXVMCache,
    hardfork_state: hardforks::HardForkState,

    chain_height: u64,
    top_block_hash: [u8; 32],
    already_generated_coins: u64,
}

enum MutexLockState {
    Locked,
    Acquiring(OwnedMutexLockFuture<InternalBlockChainContext>),
    Acquired(OwnedMutexGuard<InternalBlockChainContext>),
}
pub struct BlockChainContextService {
    internal_blockchain_context: Arc<Mutex<InternalBlockChainContext>>,
    lock_state: MutexLockState,
}

impl Clone for BlockChainContextService {
    fn clone(&self) -> Self {
        BlockChainContextService {
            internal_blockchain_context: self.internal_blockchain_context.clone(),
            lock_state: MutexLockState::Locked,
        }
    }
}

impl Service<BlockChainContextRequest> for BlockChainContextService {
    type Response = BlockChainContextResponse;
    type Error = tower::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        loop {
            match &mut self.lock_state {
                MutexLockState::Locked => {
                    self.lock_state = MutexLockState::Acquiring(
                        Arc::clone(&self.internal_blockchain_context).lock_owned(),
                    )
                }
                MutexLockState::Acquiring(lock) => {
                    self.lock_state = MutexLockState::Acquired(futures::ready!(lock.poll_unpin(cx)))
                }
                MutexLockState::Acquired(_) => return Poll::Ready(Ok(())),
            }
        }
    }

    fn call(&mut self, req: BlockChainContextRequest) -> Self::Future {
        let MutexLockState::Acquired(mut internal_blockchain_context) =
            std::mem::replace(&mut self.lock_state, MutexLockState::Locked)
        else {
            panic!("poll_ready() was not called first!")
        };
        async move {
            let InternalBlockChainContext {
                current_validity_token,
                current_reorg_token,
                difficulty_cache,
                weight_cache,
                rx_seed_cache,
                hardfork_state,
                chain_height,
                top_block_hash,
                already_generated_coins,
            } = internal_blockchain_context.deref_mut();

            let res = match req {
                BlockChainContextRequest::GetContext => {
                    let current_hf = hardfork_state.current_hardfork();

                    BlockChainContextResponse::Context(BlockChainContext {
                        validity_token: current_validity_token.clone(),
                        raw: RawBlockChainContext {
                            context_to_verify_block: ContextToVerifyBlock {
                                median_weight_for_block_reward: weight_cache
                                    .median_for_block_reward(&current_hf),
                                effective_median_weight: weight_cache
                                    .effective_median_block_weight(&current_hf),
                                top_hash: *top_block_hash,
                                median_block_timestamp: difficulty_cache.median_timestamp(
                                    usize::try_from(BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW).unwrap(),
                                ),
                                chain_height: *chain_height,
                                current_hf,
                                next_difficulty: difficulty_cache.next_difficulty(&current_hf),
                                already_generated_coins: *already_generated_coins,
                            },
                            rx_vms: rx_seed_cache.get_vms(),
                            cumulative_difficulty: difficulty_cache.cumulative_difficulty(),
                            median_long_term_weight: weight_cache.median_long_term_weight(),
                            top_block_timestamp: difficulty_cache.top_block_timestamp(),
                            re_org_token: current_reorg_token.clone(),
                        },
                    })
                }
                BlockChainContextRequest::BatchGetDifficulties(blocks) => {
                    let next_diffs = difficulty_cache
                        .next_difficulties(blocks, &hardfork_state.current_hardfork());
                    BlockChainContextResponse::BatchDifficulties(next_diffs)
                }
                BlockChainContextRequest::NewRXVM(vm) => {
                    rx_seed_cache.add_vm(vm);
                    BlockChainContextResponse::Ok
                }
                BlockChainContextRequest::Update(new) => {
                    // Cancel the validity token and replace it with a new one.
                    std::mem::replace(current_validity_token, ValidityToken::new())
                        .set_data_invalid();

                    difficulty_cache.new_block(
                        new.height,
                        new.timestamp,
                        new.cumulative_difficulty,
                    );

                    weight_cache.new_block(new.height, new.weight, new.long_term_weight);

                    hardfork_state.new_block(new.vote, new.height);

                    rx_seed_cache
                        .new_block(
                            new.height,
                            &new.new_top_hash,
                            &hardfork_state.current_hardfork(),
                        )
                        .await;

                    *chain_height = new.height + 1;
                    *top_block_hash = new.new_top_hash;
                    *already_generated_coins =
                        already_generated_coins.saturating_add(new.generated_coins);

                    BlockChainContextResponse::Ok
                }
            };

            Ok(res)
        }
        .boxed()
    }
}

//! Context Task
//!
//! This module contains the async task that handles keeping track of blockchain context.
//! It holds all the context caches and handles [`tower::Service`] requests.
//!
use std::sync::Arc;

use arc_swap::ArcSwap;
use futures::channel::oneshot;
use tokio::sync::mpsc;
use tower::ServiceExt;
use tracing::Instrument;

use cuprate_consensus_rules::blocks::ContextToVerifyBlock;
use cuprate_helper::cast::{u64_to_usize, usize_to_u64};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    Chain, HardFork,
};

use crate::{
    alt_chains::{get_alt_chain_difficulty_cache, get_alt_chain_weight_cache, AltChainMap},
    difficulty::DifficultyCache,
    hardforks::HardForkState,
    rx_vms,
    weight::BlockWeightsCache,
    BlockChainContextRequest, BlockChainContextResponse, BlockchainContext, ContextCacheError,
    ContextConfig, Database, BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW,
};

/// A request from the context service to the context task.
pub(super) struct ContextTaskRequest {
    /// The request.
    pub req: BlockChainContextRequest,
    /// The response channel.
    pub tx: oneshot::Sender<Result<BlockChainContextResponse, tower::BoxError>>,
    /// The tracing span of the requester.
    pub span: tracing::Span,
}

/// The Context task that keeps the blockchain context and handles requests.
pub(crate) struct ContextTask<D: Database> {
    context_cache: Arc<ArcSwap<BlockchainContext>>,

    /// The difficulty cache.
    difficulty_cache: DifficultyCache,
    /// The weight cache.
    weight_cache: BlockWeightsCache,
    /// The RX VM cache.
    rx_vm_cache: rx_vms::RandomXVmCache,
    /// The hard-fork state cache.
    hardfork_state: HardForkState,

    alt_chain_cache_map: AltChainMap,

    /// The current chain height.
    chain_height: usize,
    /// The top block hash.
    top_block_hash: [u8; 32],
    /// The total amount of coins generated.
    already_generated_coins: u64,

    database: D,
}

impl<D: Database + Clone + Send + 'static> ContextTask<D> {
    /// Initialize the [`ContextTask`], this will need to pull a lot of data from the database so may take a
    /// while to complete.
    pub(crate) async fn init_context(
        cfg: ContextConfig,
        mut database: D,
    ) -> Result<(Self, Arc<ArcSwap<BlockchainContext>>), ContextCacheError> {
        let ContextConfig {
            difficulty_cfg,
            weights_config,
            hard_fork_cfg,
        } = cfg;

        tracing::debug!("Initialising blockchain context");

        let BlockchainResponse::ChainHeight(chain_height, top_block_hash) = database
            .ready()
            .await?
            .call(BlockchainReadRequest::ChainHeight)
            .await?
        else {
            panic!("Database sent incorrect response!");
        };

        let BlockchainResponse::GeneratedCoins(already_generated_coins) = database
            .ready()
            .await?
            .call(BlockchainReadRequest::GeneratedCoins(chain_height - 1))
            .await?
        else {
            panic!("Database sent incorrect response!");
        };

        let db = database.clone();
        let hardfork_state_handle = tokio::spawn(async move {
            HardForkState::init_from_chain_height(chain_height, hard_fork_cfg, db).await
        });

        let db = database.clone();
        let difficulty_cache_handle = tokio::spawn(async move {
            DifficultyCache::init_from_chain_height(chain_height, difficulty_cfg, db, Chain::Main)
                .await
        });

        let db = database.clone();
        let weight_cache_handle = tokio::spawn(async move {
            BlockWeightsCache::init_from_chain_height(chain_height, weights_config, db, Chain::Main)
                .await
        });

        // Wait for the hardfork state to finish first as we need it to start the randomX VM cache.
        let hardfork_state = hardfork_state_handle.await.unwrap()?;
        let current_hf = hardfork_state.current_hardfork();

        let db = database.clone();
        let rx_seed_handle = tokio::spawn(async move {
            rx_vms::RandomXVmCache::init_from_chain_height(chain_height, &current_hf, db).await
        });

        let difficulty_cache = difficulty_cache_handle.await.unwrap()?;
        let weight_cache = weight_cache_handle.await.unwrap()?;

        let blockchain_context = blockchain_context(
            &weight_cache,
            &difficulty_cache,
            current_hf,
            top_block_hash,
            chain_height,
            already_generated_coins,
        );

        let context_cache = Arc::new(ArcSwap::from_pointee(blockchain_context));

        let context_svc = Self {
            context_cache: Arc::clone(&context_cache),
            difficulty_cache,
            weight_cache,
            rx_vm_cache: rx_seed_handle.await.unwrap()?,
            hardfork_state,
            alt_chain_cache_map: AltChainMap::new(),
            chain_height,
            already_generated_coins,
            top_block_hash,
            database,
        };

        Ok((context_svc, context_cache))
    }

    fn update_blockchain_context(&self) {
        let context = blockchain_context(
            &self.weight_cache,
            &self.difficulty_cache,
            self.hardfork_state.current_hardfork(),
            self.top_block_hash,
            self.chain_height,
            self.already_generated_coins,
        );

        self.context_cache.store(Arc::new(context));
    }

    /// Handles a [`BlockChainContextRequest`] and returns a [`BlockChainContextResponse`].
    pub(crate) async fn handle_req(
        &mut self,
        req: BlockChainContextRequest,
    ) -> Result<BlockChainContextResponse, tower::BoxError> {
        Ok(match req {
            BlockChainContextRequest::CurrentRxVms => {
                BlockChainContextResponse::RxVms(self.rx_vm_cache.get_vms().await)
            }
            BlockChainContextRequest::BatchGetDifficulties(blocks) => {
                tracing::debug!("Getting batch difficulties len: {}", blocks.len() + 1);

                let next_diffs = self
                    .difficulty_cache
                    .next_difficulties(blocks, self.hardfork_state.current_hardfork());
                BlockChainContextResponse::BatchDifficulties(next_diffs)
            }
            BlockChainContextRequest::NewRXVM(vm) => {
                tracing::debug!("Adding randomX VM to cache.");

                self.rx_vm_cache.add_vm(vm);
                BlockChainContextResponse::Ok
            }
            BlockChainContextRequest::Update(new) => {
                tracing::debug!(
                    "Updating blockchain cache with new block, height: {}",
                    new.height
                );

                self.difficulty_cache.new_block(
                    new.height,
                    new.timestamp,
                    new.cumulative_difficulty,
                );

                self.weight_cache
                    .new_block(new.height, new.weight, new.long_term_weight);

                self.hardfork_state.new_block(new.vote, new.height);

                self.rx_vm_cache.new_block(new.height, &new.block_hash);

                self.chain_height = new.height + 1;
                self.top_block_hash = new.block_hash;
                self.already_generated_coins = self
                    .already_generated_coins
                    .saturating_add(new.generated_coins);

                self.update_blockchain_context();

                BlockChainContextResponse::Ok
            }
            BlockChainContextRequest::PopBlocks { numb_blocks } => {
                assert!(numb_blocks < self.chain_height);

                self.difficulty_cache
                    .pop_blocks_main_chain(numb_blocks, self.database.clone())
                    .await?;
                self.weight_cache
                    .pop_blocks_main_chain(numb_blocks, self.database.clone())
                    .await?;
                self.rx_vm_cache
                    .pop_blocks_main_chain(self.chain_height - numb_blocks - 1);
                self.hardfork_state
                    .pop_blocks_main_chain(numb_blocks, self.database.clone())
                    .await?;

                self.alt_chain_cache_map.clear();

                self.chain_height -= numb_blocks;

                let BlockchainResponse::GeneratedCoins(already_generated_coins) = self
                    .database
                    .ready()
                    .await?
                    .call(BlockchainReadRequest::GeneratedCoins(self.chain_height - 1))
                    .await?
                else {
                    panic!("Database sent incorrect response!");
                };

                let BlockchainResponse::BlockHash(top_block_hash) = self
                    .database
                    .ready()
                    .await?
                    .call(BlockchainReadRequest::BlockHash(
                        self.chain_height - 1,
                        Chain::Main,
                    ))
                    .await?
                else {
                    panic!("Database returned incorrect response!");
                };

                self.already_generated_coins = already_generated_coins;
                self.top_block_hash = top_block_hash;

                self.update_blockchain_context();

                BlockChainContextResponse::Ok
            }
            BlockChainContextRequest::ClearAltCache => {
                self.alt_chain_cache_map.clear();

                BlockChainContextResponse::Ok
            }
            BlockChainContextRequest::AltChainContextCache { prev_id, _token } => {
                BlockChainContextResponse::AltChainContextCache(
                    self.alt_chain_cache_map
                        .get_alt_chain_context(prev_id, &mut self.database)
                        .await?,
                )
            }
            BlockChainContextRequest::AltChainDifficultyCache { prev_id, _token } => {
                BlockChainContextResponse::AltChainDifficultyCache(
                    get_alt_chain_difficulty_cache(
                        prev_id,
                        &self.difficulty_cache,
                        self.database.clone(),
                    )
                    .await?,
                )
            }
            BlockChainContextRequest::AltChainWeightCache { prev_id, _token } => {
                BlockChainContextResponse::AltChainWeightCache(
                    get_alt_chain_weight_cache(prev_id, &self.weight_cache, self.database.clone())
                        .await?,
                )
            }
            BlockChainContextRequest::AltChainRxVM {
                height,
                chain,
                _token,
            } => BlockChainContextResponse::AltChainRxVM(
                self.rx_vm_cache
                    .get_alt_vm(height, chain, &mut self.database)
                    .await?,
            ),
            BlockChainContextRequest::AddAltChainContextCache { cache, _token } => {
                self.alt_chain_cache_map.add_alt_cache(cache);
                BlockChainContextResponse::Ok
            }
            BlockChainContextRequest::HardForkInfo(hf) => {
                let state = &self.hardfork_state;

                let hf_info = state.config.info.info_for_hf(&hf);

                BlockChainContextResponse::HardForkInfo(cuprate_types::rpc::HardForkInfo {
                    earliest_height: usize_to_u64(
                        state.config.info.get_earliest_ideal_height_for_version(hf),
                    ),
                    enabled: state.current_hardfork == hf,
                    state: 2, // TODO: <https://github.com/monero-project/monero/blob/125622d5bdc42cf552be5c25009bd9ab52c0a7ca/src/cryptonote_basic/hardfork.h#L46>
                    threshold: hf_info.threshold().try_into().unwrap(),
                    version: hf.as_u8(),
                    votes: state.votes.votes_for_hf(&hf).try_into().unwrap(),
                    voting: hf.as_u8(),
                    window: state.config.window.try_into().unwrap(),
                })
            }
            BlockChainContextRequest::FeeEstimate { .. }
            | BlockChainContextRequest::AltChains
            | BlockChainContextRequest::CalculatePow { .. } => {
                todo!("finish https://github.com/Cuprate/cuprate/pull/297")
            }
        })
    }

    /// Run the [`ContextTask`], the task will listen for requests on the passed in channel. When the channel closes the
    /// task will finish.
    pub(crate) async fn run(mut self, mut rx: mpsc::Receiver<ContextTaskRequest>) {
        while let Some(req) = rx.recv().await {
            let res = self.handle_req(req.req).instrument(req.span).await;
            drop(req.tx.send(res));
        }

        tracing::info!("Shutting down blockchain context task.");
    }
}

fn blockchain_context(
    weight_cache: &BlockWeightsCache,
    difficulty_cache: &DifficultyCache,

    current_hf: HardFork,
    top_hash: [u8; 32],
    chain_height: usize,
    already_generated_coins: u64,
) -> BlockchainContext {
    BlockchainContext {
        context_to_verify_block: ContextToVerifyBlock {
            median_weight_for_block_reward: weight_cache.median_for_block_reward(current_hf),
            effective_median_weight: weight_cache.effective_median_block_weight(current_hf),
            top_hash,
            median_block_timestamp: difficulty_cache
                .median_timestamp(u64_to_usize(BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW)),
            chain_height,
            current_hf,
            next_difficulty: difficulty_cache.next_difficulty(current_hf),
            already_generated_coins,
        },
        cumulative_difficulty: difficulty_cache.cumulative_difficulty(),
        median_long_term_weight: weight_cache.median_long_term_weight(),
        top_block_timestamp: difficulty_cache.top_block_timestamp(),
    }
}

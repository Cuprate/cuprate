use futures::channel::oneshot;
use tokio::sync::mpsc::UnboundedReceiver;
use tower::ServiceExt;
use tracing::Instrument;

use cuprate_consensus_rules::blocks::ContextToVerifyBlock;

use super::{
    difficulty, hardforks, rx_vms, weight, BlockChainContext, BlockChainContextRequest,
    BlockChainContextResponse, ContextConfig, RawBlockChainContext, ReOrgToken, ValidityToken,
    BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW,
};
use crate::{Database, DatabaseRequest, DatabaseResponse, ExtendedConsensusError};

pub(super) struct ContextTaskRequest {
    pub req: BlockChainContextRequest,
    pub tx: oneshot::Sender<Result<BlockChainContextResponse, tower::BoxError>>,
    pub span: tracing::Span,
}

pub struct ContextTask {
    /// A token used to invalidate previous contexts when a new
    /// block is added to the chain.
    current_validity_token: ValidityToken,
    /// A token which is used to signal a reorg has happened.
    current_reorg_token: ReOrgToken,

    /// The difficulty cache.
    difficulty_cache: difficulty::DifficultyCache,
    /// The weight cache.
    weight_cache: weight::BlockWeightsCache,
    /// The RX VM cache.
    rx_vm_cache: rx_vms::RandomXVMCache,
    /// The hard-fork state cache.
    hardfork_state: hardforks::HardForkState,

    /// The current chain height.
    chain_height: u64,
    /// The top block hash.
    top_block_hash: [u8; 32],
    /// The total amount of coins generated.
    already_generated_coins: u64,
}

impl ContextTask {
    pub async fn init_context<D: Database>(
        cfg: ContextConfig,
        mut database: D,
    ) -> Result<ContextTask, ExtendedConsensusError>
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
            difficulty::DifficultyCache::init_from_chain_height(chain_height, difficulty_cfg, db)
                .await
        });

        let db = database.clone();
        let weight_cache_handle = tokio::spawn(async move {
            weight::BlockWeightsCache::init_from_chain_height(chain_height, weights_config, db)
                .await
        });

        // Wait for the hardfork state to finish first as we need it to start the randomX VM cache.
        let hardfork_state = hardfork_state_handle.await.unwrap()?;
        let current_hf = hardfork_state.current_hardfork();

        let db = database.clone();
        let rx_seed_handle = tokio::spawn(async move {
            rx_vms::RandomXVMCache::init_from_chain_height(chain_height, &current_hf, db).await
        });

        let context_svc = ContextTask {
            current_validity_token: ValidityToken::new(),
            current_reorg_token: ReOrgToken::new(),
            difficulty_cache: difficulty_cache_handle.await.unwrap()?,
            weight_cache: weight_cache_handle.await.unwrap()?,
            rx_vm_cache: rx_seed_handle.await.unwrap()?,
            hardfork_state,
            chain_height,
            already_generated_coins,
            top_block_hash,
        };

        Ok(context_svc)
    }

    pub async fn handle_req(
        &mut self,
        req: BlockChainContextRequest,
    ) -> Result<BlockChainContextResponse, tower::BoxError> {
        Ok(match req {
            BlockChainContextRequest::GetContext => {
                let current_hf = self.hardfork_state.current_hardfork();

                BlockChainContextResponse::Context(BlockChainContext {
                    validity_token: self.current_validity_token.clone(),
                    raw: RawBlockChainContext {
                        context_to_verify_block: ContextToVerifyBlock {
                            median_weight_for_block_reward: self
                                .weight_cache
                                .median_for_block_reward(&current_hf),
                            effective_median_weight: self
                                .weight_cache
                                .effective_median_block_weight(&current_hf),
                            top_hash: self.top_block_hash,
                            median_block_timestamp: self.difficulty_cache.median_timestamp(
                                usize::try_from(BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW).unwrap(),
                            ),
                            chain_height: self.chain_height,
                            current_hf,
                            next_difficulty: self.difficulty_cache.next_difficulty(&current_hf),
                            already_generated_coins: self.already_generated_coins,
                        },
                        rx_vms: self.rx_vm_cache.get_vms(),
                        cumulative_difficulty: self.difficulty_cache.cumulative_difficulty(),
                        median_long_term_weight: self.weight_cache.median_long_term_weight(),
                        top_block_timestamp: self.difficulty_cache.top_block_timestamp(),
                        re_org_token: self.current_reorg_token.clone(),
                    },
                })
            }
            BlockChainContextRequest::BatchGetDifficulties(blocks) => {
                let next_diffs = self
                    .difficulty_cache
                    .next_difficulties(blocks, &self.hardfork_state.current_hardfork());
                BlockChainContextResponse::BatchDifficulties(next_diffs)
            }
            BlockChainContextRequest::NewRXVM(vm) => {
                self.rx_vm_cache.add_vm(vm);
                BlockChainContextResponse::Ok
            }
            BlockChainContextRequest::Update(new) => {
                // Cancel the validity token and replace it with a new one.
                std::mem::replace(&mut self.current_validity_token, ValidityToken::new())
                    .set_data_invalid();

                self.difficulty_cache.new_block(
                    new.height,
                    new.timestamp,
                    new.cumulative_difficulty,
                );

                self.weight_cache
                    .new_block(new.height, new.weight, new.long_term_weight);

                self.hardfork_state.new_block(new.vote, new.height);

                self.rx_vm_cache
                    .new_block(
                        new.height,
                        &new.block_hash,
                        // We use the current hf and not the hf of the top block as when syncing we need to generate VMs
                        // on the switch to RX not after it.
                        &self.hardfork_state.current_hardfork(),
                    )
                    .await;

                self.chain_height = new.height + 1;
                self.top_block_hash = new.block_hash;
                self.already_generated_coins = self
                    .already_generated_coins
                    .saturating_add(new.generated_coins);

                BlockChainContextResponse::Ok
            }
        })
    }

    pub async fn run(mut self, mut rx: UnboundedReceiver<ContextTaskRequest>) {
        while let Some(req) = rx.recv().await {
            let res = self.handle_req(req.req).instrument(req.span).await;
            let _ = req.tx.send(res);
        }
    }
}

//! Block Verifier Service.
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::FutureExt;
use monero_serai::{
    block::Block,
    transaction::{Input, Transaction},
};
use tower::{Service, ServiceExt};

use cuprate_helper::asynch::rayon_spawn_async;
use cuprate_types::{
    AltBlockInformation, VerifiedBlockInformation, VerifiedTransactionInformation,
};

use cuprate_consensus_rules::{
    blocks::{
        calculate_pow_hash, check_block, check_block_pow, randomx_seed_height, BlockError, RandomX,
    },
    miner_tx::MinerTxError,
    ConsensusError, HardFork,
};

use crate::{
    context::{BlockChainContextRequest, BlockChainContextResponse, RawBlockChainContext},
    transactions::{TransactionVerificationData, VerifyTxRequest, VerifyTxResponse},
    Database, ExtendedConsensusError,
};

mod alt_block;
mod batch_prepare;
mod free;

use alt_block::sanity_check_alt_block;
use batch_prepare::batch_prepare_main_chain_block;
use cuprate_consensus_rules::hard_forks::HardForkError;
use free::pull_ordered_transactions;

/// A pre-prepared block with all data needed to verify it, except the block's proof of work.
#[derive(Debug)]
pub struct PreparedBlockExPow {
    /// The block.
    pub block: Block,
    /// The serialised block's bytes.
    pub block_blob: Vec<u8>,

    /// The block's hard-fork vote.
    pub hf_vote: HardFork,
    /// The block's hard-fork version.
    pub hf_version: HardFork,

    /// The block's hash.
    pub block_hash: [u8; 32],
    /// The height of the block.
    pub height: usize,

    /// The weight of the block's miner transaction.
    pub miner_tx_weight: usize,
}

impl PreparedBlockExPow {
    /// Prepare a new block.
    ///
    /// # Errors
    /// This errors if either the `block`'s:
    /// - Hard-fork values are invalid
    /// - Miner transaction is missing a miner input
    pub fn new(block: Block) -> Result<PreparedBlockExPow, ConsensusError> {
        let (hf_version, hf_vote) =
            HardFork::from_block_header(&block.header).map_err(|_| BlockError::HardForkError(HardForkError::HardForkUnknown))?;

        let Some(Input::Gen(height)) = block.miner_transaction.prefix().inputs.first() else {
            Err(ConsensusError::Block(BlockError::MinerTxError(
                MinerTxError::InputNotOfTypeGen,
            )))?
        };

        Ok(PreparedBlockExPow {
            block_blob: block.serialize(),
            hf_vote,
            hf_version,

            block_hash: block.hash(),
            height: *height,

            miner_tx_weight: block.miner_transaction.weight(),
            block,
        })
    }
}

/// A pre-prepared block with all data needed to verify it.
#[derive(Debug)]
pub struct PreparedBlock {
    /// The block
    pub block: Block,
    /// The serialised blocks bytes
    pub block_blob: Vec<u8>,

    /// The blocks hf vote
    pub hf_vote: HardFork,
    /// The blocks hf version
    pub hf_version: HardFork,

    /// The blocks hash
    pub block_hash: [u8; 32],
    /// The blocks POW hash.
    pub pow_hash: [u8; 32],

    /// The weight of the blocks miner transaction.
    pub miner_tx_weight: usize,
}

impl PreparedBlock {
    /// Creates a new [`PreparedBlock`].
    ///
    /// The randomX VM must be Some if RX is needed or this will panic.
    /// The randomX VM must also be initialised with the correct seed.
    fn new<R: RandomX>(
        block: Block,
        randomx_vm: Option<&R>,
    ) -> Result<PreparedBlock, ConsensusError> {
        let (hf_version, hf_vote) =
            HardFork::from_block_header(&block.header).map_err(|_| BlockError::HardForkError(HardForkError::HardForkUnknown))?;

        let [Input::Gen(height)] = &block.miner_transaction.prefix().inputs[..] else {
            Err(ConsensusError::Block(BlockError::MinerTxError(
                MinerTxError::InputNotOfTypeGen,
            )))?
        };

        Ok(PreparedBlock {
            block_blob: block.serialize(),
            hf_vote,
            hf_version,

            block_hash: block.hash(),
            pow_hash: calculate_pow_hash(
                randomx_vm,
                &block.serialize_pow_hash(),
                *height,
                &hf_version,
            )?,

            miner_tx_weight: block.miner_transaction.weight(),
            block,
        })
    }

    /// Creates a new [`PreparedBlock`] from a [`PreparedBlockExPow`].
    ///
    /// This function will give an invalid PoW hash if `randomx_vm` is not initialised
    /// with the correct seed.
    ///
    /// # Panics
    /// This function will panic if `randomx_vm` is
    /// [`None`] even though RandomX is needed.
    fn new_prepped<R: RandomX>(
        block: PreparedBlockExPow,
        randomx_vm: Option<&R>,
    ) -> Result<PreparedBlock, ConsensusError> {
        Ok(PreparedBlock {
            block_blob: block.block_blob,
            hf_vote: block.hf_vote,
            hf_version: block.hf_version,

            block_hash: block.block_hash,
            pow_hash: calculate_pow_hash(
                randomx_vm,
                &block.block.serialize_pow_hash(),
                block.height,
                &block.hf_version,
            )?,

            miner_tx_weight: block.block.miner_transaction.weight(),
            block: block.block,
        })
    }
}

/// A request to verify a block.
pub enum VerifyBlockRequest {
    /// A request to verify a block.
    MainChain {
        block: Block,
        prepared_txs: HashMap<[u8; 32], TransactionVerificationData>,
    },
    /// Verifies a prepared block.
    MainChainPrepped {
        /// The already prepared block.
        block: PreparedBlock,
        /// The full list of transactions for this block, in the order given in `block`.
        // TODO: Remove the Arc here
        txs: Vec<Arc<TransactionVerificationData>>,
    },
    /// Batch prepares a list of blocks and transactions for verification.
    MainChainBatchPrepareBlocks {
        /// The list of blocks and their transactions (not necessarily in the order given in the block).
        blocks: Vec<(Block, Vec<Transaction>)>,
    },
    /// A request to sanity check an alt block, also returning the cumulative difficulty of the alt chain.
    ///
    /// Unlike requests to verify main chain blocks, you do not need to add the returned block to the context
    /// service, you will still have to add it to the database though.
    AltChain {
        /// The alt block to sanity check.
        block: Block,
        /// The alt transactions.
        prepared_txs: HashMap<[u8; 32], TransactionVerificationData>,
    },
}

/// A response from a verify block request.
#[allow(clippy::large_enum_variant)] // The largest variant is most common ([`MainChain`])
pub enum VerifyBlockResponse {
    /// This block is valid.
    MainChain(VerifiedBlockInformation),
    /// The sanity checked alt block.
    AltChain(AltBlockInformation),
    /// A list of prepared blocks for verification, you should call [`VerifyBlockRequest::MainChainPrepped`] on each of the returned
    /// blocks to fully verify them.
    MainChainBatchPrepped(Vec<(PreparedBlock, Vec<Arc<TransactionVerificationData>>)>),
}

/// The block verifier service.
pub struct BlockVerifierService<C, TxV, D> {
    /// The context service.
    context_svc: C,
    /// The tx verifier service.
    tx_verifier_svc: TxV,
    /// The database.
    // Not use yet but will be.
    _database: D,
}

impl<C, TxV, D> BlockVerifierService<C, TxV, D>
where
    C: Service<BlockChainContextRequest, Response = BlockChainContextResponse>
        + Clone
        + Send
        + 'static,
    TxV: Service<VerifyTxRequest, Response = VerifyTxResponse, Error = ExtendedConsensusError>
        + Clone
        + Send
        + 'static,
    D: Database + Clone + Send + Sync + 'static,
    D::Future: Send + 'static,
{
    /// Creates a new block verifier.
    pub(crate) fn new(
        context_svc: C,
        tx_verifier_svc: TxV,
        database: D,
    ) -> BlockVerifierService<C, TxV, D> {
        BlockVerifierService {
            context_svc,
            tx_verifier_svc,
            _database: database,
        }
    }
}

impl<C, TxV, D> Service<VerifyBlockRequest> for BlockVerifierService<C, TxV, D>
where
    C: Service<
            BlockChainContextRequest,
            Response = BlockChainContextResponse,
            Error = tower::BoxError,
        > + Clone
        + Send
        + 'static,
    C::Future: Send + 'static,

    TxV: Service<VerifyTxRequest, Response = VerifyTxResponse, Error = ExtendedConsensusError>
        + Clone
        + Send
        + 'static,
    TxV::Future: Send + 'static,

    D: Database + Clone + Send + Sync + 'static,
    D::Future: Send + 'static,
{
    type Response = VerifyBlockResponse;
    type Error = ExtendedConsensusError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: VerifyBlockRequest) -> Self::Future {
        let context_svc = self.context_svc.clone();
        let tx_verifier_svc = self.tx_verifier_svc.clone();

        async move {
            match req {
                VerifyBlockRequest::MainChain {
                    block,
                    prepared_txs,
                } => {
                    verify_main_chain_block(block, prepared_txs, context_svc, tx_verifier_svc).await
                }
                VerifyBlockRequest::MainChainBatchPrepareBlocks { blocks } => {
                    batch_prepare_main_chain_block(blocks, context_svc).await
                }
                VerifyBlockRequest::MainChainPrepped { block, txs } => {
                    verify_prepped_main_chain_block(block, txs, context_svc, tx_verifier_svc, None)
                        .await
                }
                VerifyBlockRequest::AltChain {
                    block,
                    prepared_txs,
                } => sanity_check_alt_block(block, prepared_txs, context_svc).await,
            }
        }
        .boxed()
    }
}

/// Verifies a prepared block.
async fn verify_main_chain_block<C, TxV>(
    block: Block,
    txs: HashMap<[u8; 32], TransactionVerificationData>,
    mut context_svc: C,
    tx_verifier_svc: TxV,
) -> Result<VerifyBlockResponse, ExtendedConsensusError>
where
    C: Service<
            BlockChainContextRequest,
            Response = BlockChainContextResponse,
            Error = tower::BoxError,
        > + Send
        + 'static,
    C::Future: Send + 'static,
    TxV: Service<VerifyTxRequest, Response = VerifyTxResponse, Error = ExtendedConsensusError>,
{
    let BlockChainContextResponse::Context(checked_context) = context_svc
        .ready()
        .await?
        .call(BlockChainContextRequest::GetContext)
        .await?
    else {
        panic!("Context service returned wrong response!");
    };

    let context = checked_context.unchecked_blockchain_context().clone();
    tracing::debug!("got blockchain context: {:?}", context);

    tracing::debug!(
        "Preparing block for verification, expected height: {}",
        context.chain_height
    );

    // Set up the block and just pass it to [`verify_prepped_main_chain_block`]

    // We just use the raw `hardfork_version` here, no need to turn it into a `HardFork`.
    let rx_vms = if block.header.hardfork_version < 12 {
        HashMap::new()
    } else {
        let BlockChainContextResponse::RxVms(rx_vms) = context_svc
            .ready()
            .await?
            .call(BlockChainContextRequest::GetCurrentRxVm)
            .await?
        else {
            panic!("Blockchain context service returned wrong response!");
        };

        rx_vms
    };

    let height = context.chain_height;
    let prepped_block = rayon_spawn_async(move || {
        PreparedBlock::new(
            block,
            rx_vms.get(&randomx_seed_height(height)).map(AsRef::as_ref),
        )
    })
    .await?;

    check_block_pow(&prepped_block.pow_hash, context.next_difficulty)
        .map_err(ConsensusError::Block)?;

    // Check that the txs included are what we need and that there are not any extra.
    // TODO: Remove the Arc here
    let ordered_txs = pull_ordered_transactions(&prepped_block.block, txs)?
        .into_iter()
        .map(Arc::new)
        .collect();

    verify_prepped_main_chain_block(
        prepped_block,
        ordered_txs,
        context_svc,
        tx_verifier_svc,
        Some(context),
    )
    .await
}

async fn verify_prepped_main_chain_block<C, TxV>(
    prepped_block: PreparedBlock,
    txs: Vec<Arc<TransactionVerificationData>>,
    context_svc: C,
    tx_verifier_svc: TxV,
    cached_context: Option<RawBlockChainContext>,
) -> Result<VerifyBlockResponse, ExtendedConsensusError>
where
    C: Service<
            BlockChainContextRequest,
            Response = BlockChainContextResponse,
            Error = tower::BoxError,
        > + Send
        + 'static,
    C::Future: Send + 'static,
    TxV: Service<VerifyTxRequest, Response = VerifyTxResponse, Error = ExtendedConsensusError>,
{
    let context = if let Some(context) = cached_context {
        context
    } else {
        let BlockChainContextResponse::Context(checked_context) = context_svc
            .oneshot(BlockChainContextRequest::GetContext)
            .await?
        else {
            panic!("Context service returned wrong response!");
        };

        let context = checked_context.unchecked_blockchain_context().clone();

        tracing::debug!("got blockchain context: {context:?}");

        context
    };

    tracing::debug!("verifying block: {}", hex::encode(prepped_block.block_hash));

    check_block_pow(&prepped_block.pow_hash, context.next_difficulty)
        .map_err(ConsensusError::Block)?;

    if prepped_block.block.transactions.len() != txs.len() {
        return Err(ExtendedConsensusError::TxsIncludedWithBlockIncorrect);
    }

    if !prepped_block.block.transactions.is_empty() {
        for (expected_tx_hash, tx) in prepped_block.block.transactions.iter().zip(txs.iter()) {
            if expected_tx_hash != &tx.tx_hash {
                return Err(ExtendedConsensusError::TxsIncludedWithBlockIncorrect);
            }
        }

        tx_verifier_svc
            .oneshot(VerifyTxRequest::Prepped {
                txs: txs.clone(),
                current_chain_height: context.chain_height,
                top_hash: context.top_hash,
                time_for_time_lock: context.current_adjusted_timestamp_for_time_lock(),
                hf: context.current_hf,
            })
            .await?;
    }

    let block_weight =
        prepped_block.miner_tx_weight + txs.iter().map(|tx| tx.tx_weight).sum::<usize>();
    let total_fees = txs.iter().map(|tx| tx.fee).sum::<u64>();

    tracing::debug!("Verifying block header.");
    let (_, generated_coins) = check_block(
        &prepped_block.block,
        total_fees,
        block_weight,
        prepped_block.block_blob.len(),
        &context.context_to_verify_block,
    )
    .map_err(ConsensusError::Block)?;

    Ok(VerifyBlockResponse::MainChain(VerifiedBlockInformation {
        block_hash: prepped_block.block_hash,
        block: prepped_block.block,
        block_blob: prepped_block.block_blob,
        txs: txs
            .into_iter()
            .map(|tx| {
                // Note: it would be possible for the transaction verification service to hold onto the tx after the call
                // if one of txs was invalid and the rest are still in rayon threads.
                let tx = Arc::into_inner(tx).expect(
                    "Transaction verification service should not hold onto valid transactions.",
                );

                VerifiedTransactionInformation {
                    tx_blob: tx.tx_blob,
                    tx_weight: tx.tx_weight,
                    fee: tx.fee,
                    tx_hash: tx.tx_hash,
                    tx: tx.tx,
                }
            })
            .collect(),
        pow_hash: prepped_block.pow_hash,
        generated_coins,
        weight: block_weight,
        height: context.chain_height,
        long_term_weight: context.next_block_long_term_weight(block_weight),
        cumulative_difficulty: context.cumulative_difficulty + context.next_difficulty,
    }))
}

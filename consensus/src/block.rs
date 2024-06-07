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
use rayon::prelude::*;
use tower::{Service, ServiceExt};
use tracing::instrument;

use cuprate_consensus_rules::{
    blocks::{
        calculate_pow_hash, check_block, check_block_pow, is_randomx_seed_height,
        randomx_seed_height, BlockError, RandomX,
    },
    miner_tx::MinerTxError,
    ConsensusError, HardFork,
};
use cuprate_helper::asynch::rayon_spawn_async;
use cuprate_types::{VerifiedBlockInformation, VerifiedTransactionInformation};

use crate::{
    context::{
        rx_vms::RandomXVM, BlockChainContextRequest, BlockChainContextResponse,
        RawBlockChainContext,
    },
    transactions::{TransactionVerificationData, VerifyTxRequest, VerifyTxResponse},
    Database, ExtendedConsensusError,
};

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
    pub height: u64,

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
            HardFork::from_block_header(&block.header).map_err(BlockError::HardForkError)?;

        let Some(Input::Gen(height)) = block.miner_tx.prefix.inputs.first() else {
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

            miner_tx_weight: block.miner_tx.weight(),
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
            HardFork::from_block_header(&block.header).map_err(BlockError::HardForkError)?;

        let Some(Input::Gen(height)) = block.miner_tx.prefix.inputs.first() else {
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
                &block.serialize_hashable(),
                *height,
                &hf_version,
            )?,

            miner_tx_weight: block.miner_tx.weight(),
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
                &block.block.serialize_hashable(),
                block.height,
                &block.hf_version,
            )?,

            miner_tx_weight: block.block.miner_tx.weight(),
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
        txs: Vec<Arc<TransactionVerificationData>>,
    },
    /// Batch prepares a list of blocks and transactions for verification.
    MainChainBatchPrepareBlocks {
        /// The list of blocks and their transactions (not necessarily in the order given in the block).
        blocks: Vec<(Block, Vec<Transaction>)>,
    },
}

/// A response from a verify block request.
#[allow(clippy::large_enum_variant)] // The largest variant is most common ([`MainChain`])
pub enum VerifyBlockResponse {
    /// This block is valid.
    MainChain(VerifiedBlockInformation),
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
            }
        }
        .boxed()
    }
}

/// Batch prepares a list of blocks for verification.
#[instrument(level = "debug", name = "batch_prep_blocks", skip_all, fields(amt = blocks.len()))]
async fn batch_prepare_main_chain_block<C>(
    blocks: Vec<(Block, Vec<Transaction>)>,
    mut context_svc: C,
) -> Result<VerifyBlockResponse, ExtendedConsensusError>
where
    C: Service<
            BlockChainContextRequest,
            Response = BlockChainContextResponse,
            Error = tower::BoxError,
        > + Send
        + 'static,
    C::Future: Send + 'static,
{
    let (blocks, txs): (Vec<_>, Vec<_>) = blocks.into_iter().unzip();

    tracing::debug!("Calculating block hashes.");
    let blocks: Vec<PreparedBlockExPow> = rayon_spawn_async(|| {
        blocks
            .into_iter()
            .map(PreparedBlockExPow::new)
            .collect::<Result<Vec<_>, _>>()
    })
    .await?;

    // A Vec of (timestamp, HF) for each block to calculate the expected difficulty for each block.
    let mut timestamps_hfs = Vec::with_capacity(blocks.len());
    let mut new_rx_vm = None;

    tracing::debug!("Checking blocks follow each other.");

    // For every block make sure they have the correct height and previous ID
    for window in blocks.windows(2) {
        let block_0 = &window[0];
        let block_1 = &window[1];

        if block_0.block_hash != block_1.block.header.previous
            || block_0.height != block_1.height - 1
        {
            tracing::debug!("Blocks do not follow each other, verification failed.");
            Err(ConsensusError::Block(BlockError::PreviousIDIncorrect))?;
        }

        // Cache any potential RX VM seeds as we may need them for future blocks in the batch.
        if is_randomx_seed_height(block_0.height) {
            new_rx_vm = Some((block_0.height, block_0.block_hash));
        }

        timestamps_hfs.push((block_0.block.header.timestamp, block_0.hf_version))
    }

    // Get the current blockchain context.
    let BlockChainContextResponse::Context(checked_context) = context_svc
        .ready()
        .await?
        .call(BlockChainContextRequest::GetContext)
        .await
        .map_err(Into::<ExtendedConsensusError>::into)?
    else {
        panic!("Context service returned wrong response!");
    };

    // Calculate the expected difficulties for each block in the batch.
    let BlockChainContextResponse::BatchDifficulties(difficulties) = context_svc
        .ready()
        .await?
        .call(BlockChainContextRequest::BatchGetDifficulties(
            timestamps_hfs,
        ))
        .await
        .map_err(Into::<ExtendedConsensusError>::into)?
    else {
        panic!("Context service returned wrong response!");
    };

    let context = checked_context.unchecked_blockchain_context().clone();

    // Make sure the blocks follow the main chain.

    if context.chain_height != blocks[0].height {
        tracing::debug!("Blocks do not follow main chain, verification failed.");

        Err(ConsensusError::Block(BlockError::MinerTxError(
            MinerTxError::InputsHeightIncorrect,
        )))?;
    }

    if context.top_hash != blocks[0].block.header.previous {
        tracing::debug!("Blocks do not follow main chain, verification failed.");

        Err(ConsensusError::Block(BlockError::PreviousIDIncorrect))?;
    }

    let mut rx_vms = context.rx_vms;

    // If we have a RX seed in the batch calculate it.
    if let Some((new_vm_height, new_vm_seed)) = new_rx_vm {
        tracing::debug!("New randomX seed in batch, initialising VM");

        let new_vm = rayon_spawn_async(move || {
            Arc::new(RandomXVM::new(&new_vm_seed).expect("RandomX VM gave an error on set up!"))
        })
        .await;

        context_svc
            .ready()
            .await?
            .call(BlockChainContextRequest::NewRXVM((
                new_vm_seed,
                new_vm.clone(),
            )))
            .await
            .map_err(Into::<ExtendedConsensusError>::into)?;

        rx_vms.insert(new_vm_height, new_vm);
    }

    tracing::debug!("Calculating PoW and prepping transaction");

    let blocks = rayon_spawn_async(move || {
        blocks
            .into_par_iter()
            .zip(difficulties)
            .zip(txs)
            .map(|((block, difficultly), txs)| {
                // Calculate the PoW for the block.
                let height = block.height;
                let block = PreparedBlock::new_prepped(
                    block,
                    rx_vms.get(&randomx_seed_height(height)).map(AsRef::as_ref),
                )?;

                // Check the PoW
                check_block_pow(&block.pow_hash, difficultly).map_err(ConsensusError::Block)?;

                // Now setup the txs.
                let mut txs = txs
                    .into_par_iter()
                    .map(|tx| {
                        let tx = TransactionVerificationData::new(tx)?;
                        Ok::<_, ConsensusError>((tx.tx_hash, tx))
                    })
                    .collect::<Result<HashMap<_, _>, _>>()?;

                // Order the txs correctly.
                let mut ordered_txs = Vec::with_capacity(txs.len());

                for tx_hash in &block.block.txs {
                    let tx = txs
                        .remove(tx_hash)
                        .ok_or(ExtendedConsensusError::TxsIncludedWithBlockIncorrect)?;
                    ordered_txs.push(Arc::new(tx));
                }

                Ok((block, ordered_txs))
            })
            .collect::<Result<Vec<_>, ExtendedConsensusError>>()
    })
    .await?;

    Ok(VerifyBlockResponse::MainChainBatchPrepped(blocks))
}

/// Verifies a prepared block.
async fn verify_main_chain_block<C, TxV>(
    block: Block,
    mut txs: HashMap<[u8; 32], TransactionVerificationData>,
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

    let rx_vms = context.rx_vms.clone();

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

    let mut ordered_txs = Vec::with_capacity(txs.len());

    tracing::debug!("Ordering transactions for block.");

    if !prepped_block.block.txs.is_empty() {
        for tx_hash in &prepped_block.block.txs {
            let tx = txs
                .remove(tx_hash)
                .ok_or(ExtendedConsensusError::TxsIncludedWithBlockIncorrect)?;
            ordered_txs.push(Arc::new(tx));
        }
        drop(txs);
    }

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
            .await
            .map_err(Into::<ExtendedConsensusError>::into)?
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

    if prepped_block.block.txs.len() != txs.len() {
        return Err(ExtendedConsensusError::TxsIncludedWithBlockIncorrect);
    }

    if !prepped_block.block.txs.is_empty() {
        for (expected_tx_hash, tx) in prepped_block.block.txs.iter().zip(txs.iter()) {
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

//! Block Verifier Service.
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use cuprate_helper::asynch::rayon_spawn_async;
use futures::FutureExt;
use monero_serai::{block::Block, transaction::Input};
use tower::{Service, ServiceExt};

use cuprate_consensus_rules::{
    blocks::{calculate_pow_hash, check_block, check_block_pow, BlockError, RandomX},
    miner_tx::MinerTxError,
    ConsensusError, HardFork,
};
use cuprate_types::{VerifiedBlockInformation, VerifiedTransactionInformation};

use crate::{
    context::{BlockChainContextRequest, BlockChainContextResponse},
    transactions::{TransactionVerificationData, VerifyTxRequest, VerifyTxResponse},
    Database, ExtendedConsensusError,
};

/// A pre-prepared block with all data needed to verify it.
#[derive(Debug)]
pub struct PrePreparedBlock {
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

impl PrePreparedBlock {
    /// Creates a new [`PrePreparedBlock`].
    ///
    /// The randomX VM must be Some if RX is needed or this will panic.
    /// The randomX VM must also be initialised with the correct seed.
    fn new<R: RandomX>(
        block: Block,
        randomx_vm: Option<&R>,
    ) -> Result<PrePreparedBlock, ConsensusError> {
        let (hf_version, hf_vote) =
            HardFork::from_block_header(&block.header).map_err(BlockError::HardForkError)?;

        let Some(Input::Gen(height)) = block.miner_tx.prefix.inputs.first() else {
            Err(ConsensusError::Block(BlockError::MinerTxError(
                MinerTxError::InputNotOfTypeGen,
            )))?
        };

        Ok(PrePreparedBlock {
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
}

/// A request to verify a block.
pub enum VerifyBlockRequest {
    /// A request to verify a block.
    MainChain {
        block: Block,
        prepared_txs: HashMap<[u8; 32], TransactionVerificationData>,
    },
}

/// A response from a verify block request.
pub enum VerifyBlockResponse {
    /// This block is valid.
    MainChain(VerifiedBlockInformation),
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
            }
        }
        .boxed()
    }
}

/// Verifies a prepared block.
async fn verify_main_chain_block<C, TxV>(
    block: Block,
    mut txs: HashMap<[u8; 32], TransactionVerificationData>,
    context_svc: C,
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
    tracing::debug!("getting blockchain context");

    let BlockChainContextResponse::Context(checked_context) = context_svc
        .oneshot(BlockChainContextRequest::GetContext)
        .await
        .map_err(Into::<ExtendedConsensusError>::into)?
    else {
        panic!("Context service returned wrong response!");
    };

    let context = checked_context.unchecked_blockchain_context().clone();
    tracing::debug!("got blockchain context: {:?}", context);

    // Set up the block and just pass it to [`verify_main_chain_block_prepared`]

    let rx_vms = context.rx_vms.clone();

    let height = context.chain_height;
    let prepped_block = rayon_spawn_async(move || {
        PrePreparedBlock::new(block, rx_vms.get(&height).map(AsRef::as_ref))
    })
    .await?;

    tracing::debug!("verifying block: {}", hex::encode(prepped_block.block_hash));

    check_block_pow(&prepped_block.pow_hash, context.next_difficulty)
        .map_err(ConsensusError::Block)?;

    // Check that the txs included are what we need and that there are not any extra.

    let mut ordered_txs = Vec::with_capacity(txs.len());

    tracing::debug!("Checking we have correct transactions for block.");

    for tx_hash in &prepped_block.block.txs {
        let tx = txs
            .remove(tx_hash)
            .ok_or(ExtendedConsensusError::TxsIncludedWithBlockIncorrect)?;
        ordered_txs.push(Arc::new(tx));
    }
    drop(txs);

    tracing::debug!("Verifying transactions for block.");

    tx_verifier_svc
        .oneshot(VerifyTxRequest::Prepped {
            txs: ordered_txs.clone(),
            current_chain_height: context.chain_height,
            top_hash: context.top_hash,
            time_for_time_lock: context.current_adjusted_timestamp_for_time_lock(),
            hf: context.current_hf,
        })
        .await?;

    let block_weight =
        prepped_block.miner_tx_weight + ordered_txs.iter().map(|tx| tx.tx_weight).sum::<usize>();
    let total_fees = ordered_txs.iter().map(|tx| tx.fee).sum::<u64>();

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
        txs: ordered_txs
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

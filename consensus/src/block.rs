use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::FutureExt;
use monero_serai::{block::Block, transaction::Transaction};
use tower::{Service, ServiceExt};

use crate::{
    context::{BlockChainContext, BlockChainContextRequest},
    transactions::{TransactionVerificationData, VerifyTxRequest, VerifyTxResponse},
    ConsensusError, HardFork,
};

mod checks;
mod hash_worker;
mod miner_tx;

#[derive(Debug)]
pub struct VerifiedBlockInformation {
    pub block: Block,
    pub hf_vote: HardFork,
    pub txs: Vec<Arc<TransactionVerificationData>>,
    pub block_hash: [u8; 32],
    pub pow_hash: [u8; 32],
    pub height: u64,
    pub generated_coins: u64,
    pub weight: usize,
    pub long_term_weight: usize,
    pub cumulative_difficulty: u128,
}

pub enum VerifyBlockRequest {
    MainChainBatchSetupVerify(Block, Vec<Transaction>),
    MainChain(Block, Vec<Arc<TransactionVerificationData>>),
}

pub enum VerifyBlockResponse {
    MainChainBatchSetupVerify(),
}

// TODO: it is probably a bad idea for this to derive clone, if 2 places (RPC, P2P) receive valid but different blocks
// then they will both get approved but only one should go to main chain.
#[derive(Clone)]
pub struct BlockVerifierService<C: Clone, Tx: Clone> {
    context_svc: C,
    tx_verifier_svc: Tx,
}

impl<C, Tx> BlockVerifierService<C, Tx>
where
    C: Service<BlockChainContextRequest, Response = BlockChainContext> + Clone + Send + 'static,
    Tx: Service<VerifyTxRequest, Response = VerifyTxResponse, Error = ConsensusError>
        + Clone
        + Send
        + 'static,
{
    pub fn new(context_svc: C, tx_verifier_svc: Tx) -> BlockVerifierService<C, Tx> {
        BlockVerifierService {
            context_svc,
            tx_verifier_svc,
        }
    }
}

impl<C, Tx> Service<VerifyBlockRequest> for BlockVerifierService<C, Tx>
where
    C: Service<BlockChainContextRequest, Response = BlockChainContext, Error = tower::BoxError>
        + Clone
        + Send
        + 'static,
    C::Future: Send + 'static,
    Tx: Service<VerifyTxRequest, Response = VerifyTxResponse, Error = ConsensusError>
        + Clone
        + Send
        + 'static,
    Tx::Future: Send + 'static,
{
    type Response = VerifiedBlockInformation;
    type Error = ConsensusError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        futures::ready!(self.context_svc.poll_ready(cx)).map(Into::into)?;
        self.tx_verifier_svc.poll_ready(cx)
    }

    fn call(&mut self, req: VerifyBlockRequest) -> Self::Future {
        let context_svc = self.context_svc.clone();
        let tx_verifier_svc = self.tx_verifier_svc.clone();

        async move {
            match req {
                VerifyBlockRequest::MainChainBatchSetupVerify(block, txs) => {
                    batch_setup_verify_main_chain_block(block, txs, context_svc, tx_verifier_svc)
                        .await
                }
                VerifyBlockRequest::MainChain(block, txs) => {
                    verify_main_chain_block(block, txs, context_svc, tx_verifier_svc).await
                }
            }
        }
        .boxed()
    }
}

async fn verify_main_chain_block<C, Tx>(
    block: Block,
    txs: Vec<Arc<TransactionVerificationData>>,
    context_svc: C,
    tx_verifier_svc: Tx,
) -> Result<VerifiedBlockInformation, ConsensusError>
where
    C: Service<BlockChainContextRequest, Response = BlockChainContext, Error = tower::BoxError>
        + Send
        + 'static,
    C::Future: Send + 'static,
    Tx: Service<VerifyTxRequest, Response = VerifyTxResponse, Error = ConsensusError>,
{
    tracing::debug!("getting blockchain context");
    let checked_context = context_svc
        .oneshot(BlockChainContextRequest)
        .await
        .map_err(Into::<ConsensusError>::into)?;

    // TODO: should we unwrap here, we did just get the data so it should be ok.
    let context = checked_context.blockchain_context().unwrap();

    tracing::debug!("got blockchain context: {:?}", context);

    let block_weight = block.miner_tx.weight() + txs.iter().map(|tx| tx.tx_weight).sum::<usize>();
    let total_fees = txs.iter().map(|tx| tx.fee).sum::<u64>();

    tx_verifier_svc
        .oneshot(VerifyTxRequest::Block {
            txs: txs.clone(),
            current_chain_height: context.chain_height,
            time_for_time_lock: context.current_adjusted_timestamp_for_time_lock(),
            hf: context.current_hard_fork,
        })
        .await?;

    let generated_coins = miner_tx::check_miner_tx(
        &block.miner_tx,
        total_fees,
        context.chain_height,
        block_weight,
        context.median_weight_for_block_reward,
        context.already_generated_coins,
        &context.current_hard_fork,
    )?;

    let hashing_blob = block.serialize_hashable();

    checks::block_size_sanity_check(block.serialize().len(), context.effective_median_weight)?;
    checks::block_weight_check(block_weight, context.median_weight_for_block_reward)?;

    checks::check_amount_txs(block.txs.len())?;
    checks::check_prev_id(&block, &context.top_hash)?;
    if let Some(median_timestamp) = context.median_block_timestamp {
        // will only be None for the first 60 blocks
        checks::check_timestamp(&block, median_timestamp)?;
    }

    // do POW test last
    let pow_hash = tokio::task::spawn_blocking(move || {
        hash_worker::calculate_pow_hash(
            &hashing_blob,
            context.chain_height,
            &context.current_hard_fork,
        )
    })
    .await
    .unwrap()?;

    checks::check_block_pow(&pow_hash, context.next_difficulty)?;

    context
        .current_hard_fork
        .check_block_version_vote(&block.header)?;

    Ok(VerifiedBlockInformation {
        block_hash: block.hash(),
        block,
        txs,
        pow_hash,
        generated_coins,
        weight: block_weight,
        height: context.chain_height,
        long_term_weight: context.next_block_long_term_weight(block_weight),
        hf_vote: HardFork::V1,
        cumulative_difficulty: context.cumulative_difficulty + context.next_difficulty,
    })
}

async fn batch_setup_verify_main_chain_block<C, Tx>(
    block: Block,
    txs: Vec<Transaction>,
    context_svc: C,
    tx_verifier_svc: Tx,
) -> Result<VerifiedBlockInformation, ConsensusError>
where
    C: Service<BlockChainContextRequest, Response = BlockChainContext, Error = tower::BoxError>
        + Send
        + 'static,
    C::Future: Send + 'static,
    Tx: Service<VerifyTxRequest, Response = VerifyTxResponse, Error = ConsensusError>,
{
    tracing::debug!("getting blockchain context");
    let checked_context = context_svc
        .oneshot(BlockChainContextRequest)
        .await
        .map_err(Into::<ConsensusError>::into)?;

    // TODO: should we unwrap here, we did just get the data so it should be ok.
    let context = checked_context.blockchain_context().unwrap();

    tracing::debug!("got blockchain context: {:?}", context);

    // TODO: reorder these tests so we do the cheap tests first.

    let txs = if !txs.is_empty() {
        let VerifyTxResponse::BatchSetupOk(txs) = tx_verifier_svc
            .oneshot(VerifyTxRequest::BatchSetupVerifyBlock {
                txs,
                current_chain_height: context.chain_height,
                time_for_time_lock: context.current_adjusted_timestamp_for_time_lock(),
                hf: context.current_hard_fork,
            })
            .await?
        else {
            panic!("tx verifier sent incorrect response!");
        };
        txs
    } else {
        vec![]
    };

    let block_weight = block.miner_tx.weight() + txs.iter().map(|tx| tx.tx_weight).sum::<usize>();
    let total_fees = txs.iter().map(|tx| tx.fee).sum::<u64>();

    let generated_coins = miner_tx::check_miner_tx(
        &block.miner_tx,
        total_fees,
        context.chain_height,
        block_weight,
        context.median_weight_for_block_reward,
        context.already_generated_coins,
        &context.current_hard_fork,
    )?;

    let hashing_blob = block.serialize_hashable();

    checks::block_size_sanity_check(block.serialize().len(), context.effective_median_weight)?;
    checks::block_weight_check(block_weight, context.median_weight_for_block_reward)?;

    checks::check_amount_txs(block.txs.len())?;
    checks::check_prev_id(&block, &context.top_hash)?;
    if let Some(median_timestamp) = context.median_block_timestamp {
        // will only be None for the first 60 blocks
        checks::check_timestamp(&block, median_timestamp)?;
    }

    // do POW test last
    let pow_hash = tokio::task::spawn_blocking(move || {
        hash_worker::calculate_pow_hash(
            &hashing_blob,
            context.chain_height,
            &context.current_hard_fork,
        )
    })
    .await
    .unwrap()?;

    checks::check_block_pow(&pow_hash, context.next_difficulty)?;

    context
        .current_hard_fork
        .check_block_version_vote(&block.header)?;

    Ok(VerifiedBlockInformation {
        block_hash: block.hash(),
        block,
        txs,
        pow_hash,
        generated_coins,
        weight: block_weight,
        height: context.chain_height,
        long_term_weight: context.next_block_long_term_weight(block_weight),
        hf_vote: HardFork::V1,
        cumulative_difficulty: context.cumulative_difficulty + context.next_difficulty,
    })
}

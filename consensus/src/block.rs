use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::FutureExt;
use monero_serai::{block::Block, transaction::Input};
use rayon::prelude::*;
use tower::{Service, ServiceExt};

use crate::{
    context::{BlockChainContext, BlockChainContextRequest},
    helper::rayon_spawn_async,
    transactions::{TransactionVerificationData, VerifyTxRequest, VerifyTxResponse},
    ConsensusError, HardFork, TxNotInPool, TxPoolRequest, TxPoolResponse,
};

mod checks;
mod hash_worker;
mod miner_tx;

use hash_worker::calculate_pow_hash;

#[derive(Debug)]
pub struct PrePreparedBlock {
    pub block: Block,
    pub block_blob: Vec<u8>,

    pub hf_vote: HardFork,
    pub hf_version: HardFork,

    pub block_hash: [u8; 32],
    pub pow_hash: [u8; 32],

    pub miner_tx_weight: usize,
}

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
    MainChain(Block),

    BatchSetup(Vec<Block>),
    MainChainPreparedBlock(PrePreparedBlock),
}

pub enum VerifyBlockResponse {
    MainChain(VerifiedBlockInformation),

    BatchSetup(Vec<PrePreparedBlock>),
}

// TODO: it is probably a bad idea for this to derive clone, if 2 places (RPC, P2P) receive valid but different blocks
// then they will both get approved but only one should go to main chain.
#[derive(Clone)]
pub struct BlockVerifierService<C: Clone, TxV: Clone, TxP: Clone> {
    context_svc: C,
    tx_verifier_svc: TxV,
    tx_pool: TxP,
}

impl<C, TxV, TxP> BlockVerifierService<C, TxV, TxP>
where
    C: Service<BlockChainContextRequest, Response = BlockChainContext> + Clone + Send + 'static,
    TxV: Service<VerifyTxRequest, Response = VerifyTxResponse, Error = ConsensusError>
        + Clone
        + Send
        + 'static,
    TxP: Service<TxPoolRequest, Response = TxPoolResponse, Error = TxNotInPool>
        + Clone
        + Send
        + 'static,
{
    pub fn new(
        context_svc: C,
        tx_verifier_svc: TxV,
        tx_pool: TxP,
    ) -> BlockVerifierService<C, TxV, TxP> {
        BlockVerifierService {
            context_svc,
            tx_verifier_svc,
            tx_pool,
        }
    }
}

impl<C, TxV, TxP> Service<VerifyBlockRequest> for BlockVerifierService<C, TxV, TxP>
where
    C: Service<BlockChainContextRequest, Response = BlockChainContext, Error = tower::BoxError>
        + Clone
        + Send
        + 'static,
    C::Future: Send + 'static,

    TxV: Service<VerifyTxRequest, Response = VerifyTxResponse, Error = ConsensusError>
        + Clone
        + Send
        + 'static,
    TxV::Future: Send + 'static,

    TxP: Service<TxPoolRequest, Response = TxPoolResponse, Error = TxNotInPool>
        + Clone
        + Send
        + 'static,
    TxP::Future: Send + 'static,
{
    type Response = VerifyBlockResponse;
    type Error = ConsensusError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        futures::ready!(self.context_svc.poll_ready(cx)).map(Into::into)?;
        self.tx_verifier_svc.poll_ready(cx)
    }

    fn call(&mut self, req: VerifyBlockRequest) -> Self::Future {
        let context_svc = self.context_svc.clone();
        let context_svc = std::mem::replace(&mut self.context_svc, context_svc);
        let tx_verifier_svc = self.tx_verifier_svc.clone();
        let tx_pool = self.tx_pool.clone();

        async move {
            match req {
                VerifyBlockRequest::MainChain(block) => {
                    verify_main_chain_block(block, context_svc, tx_verifier_svc, tx_pool).await
                }
                VerifyBlockRequest::BatchSetup(blocks) => batch_prepare_block(blocks).await,
                VerifyBlockRequest::MainChainPreparedBlock(block) => {
                    verify_prepared_main_chain_block(block, context_svc, tx_verifier_svc, tx_pool)
                        .await
                }
            }
        }
        .boxed()
    }
}

async fn batch_prepare_block(blocks: Vec<Block>) -> Result<VerifyBlockResponse, ConsensusError> {
    Ok(VerifyBlockResponse::BatchSetup(
        rayon_spawn_async(move || {
            blocks
                .into_par_iter()
                .map(prepare_block)
                .collect::<Result<Vec<_>, _>>()
        })
        .await?,
    ))
}

fn prepare_block(block: Block) -> Result<PrePreparedBlock, ConsensusError> {
    let hf_version = HardFork::from_version(&block.header.major_version)?;
    let hf_vote = HardFork::from_vote(&block.header.major_version);

    let height = match block.miner_tx.prefix.inputs.get(0) {
        Some(Input::Gen(height)) => *height,
        _ => {
            return Err(ConsensusError::MinerTransaction(
                "Input is not a miner input",
            ))
        }
    };

    let block_hashing_blob = block.serialize_hashable();
    let (pow_hash, mut prepared_block) = rayon::join(
        || {
            // we calculate the POW hash on a different task because this takes a massive amount of time.
            calculate_pow_hash(&block_hashing_blob, height, &hf_version)
        },
        || {
            PrePreparedBlock {
                block_blob: block.serialize(),
                block_hash: block.hash(),
                // set a dummy pow hash for now. We use u8::MAX so if something odd happens and this value isn't changed it will fail for
                // difficulties > 1.
                pow_hash: [u8::MAX; 32],
                miner_tx_weight: block.miner_tx.weight(),
                block,
                hf_vote,
                hf_version,
            }
        },
    );

    prepared_block.pow_hash = pow_hash?;

    tracing::debug!("prepared block: {}", height);

    Ok(prepared_block)
}

async fn verify_prepared_main_chain_block<C, TxV, TxP>(
    block: PrePreparedBlock,
    context_svc: C,
    tx_verifier_svc: TxV,
    tx_pool: TxP,
) -> Result<VerifyBlockResponse, ConsensusError>
where
    C: Service<BlockChainContextRequest, Response = BlockChainContext, Error = tower::BoxError>
        + Send
        + 'static,
    C::Future: Send + 'static,
    TxV: Service<VerifyTxRequest, Response = VerifyTxResponse, Error = ConsensusError>,
    TxP: Service<TxPoolRequest, Response = TxPoolResponse, Error = TxNotInPool>
        + Clone
        + Send
        + 'static,
{
    tracing::debug!("getting blockchain context");
    let checked_context = context_svc
        .oneshot(BlockChainContextRequest)
        .await
        .map_err(Into::<ConsensusError>::into)?;

    // TODO: should we unwrap here, we did just get the data so it should be ok.
    let context = checked_context.unchecked_blockchain_context().clone();

    tracing::debug!("got blockchain context: {:?}", context);

    let TxPoolResponse::Transactions(txs) = tx_pool
        .oneshot(TxPoolRequest::Transactions(block.block.txs.clone()))
        .await?;

    let block_weight = block.miner_tx_weight + txs.iter().map(|tx| tx.tx_weight).sum::<usize>();
    let total_fees = txs.iter().map(|tx| tx.fee).sum::<u64>();

    tx_verifier_svc
        .oneshot(VerifyTxRequest::Block {
            txs: txs.clone(),
            current_chain_height: context.chain_height,
            time_for_time_lock: context.current_adjusted_timestamp_for_time_lock(),
            hf: context.current_hard_fork,
            re_org_token: context.re_org_token.clone(),
        })
        .await?;

    let generated_coins = miner_tx::check_miner_tx(
        &block.block.miner_tx,
        total_fees,
        context.chain_height,
        block_weight,
        context.median_weight_for_block_reward,
        context.already_generated_coins,
        &context.current_hard_fork,
    )?;

    checks::block_size_sanity_check(block.block_blob.len(), context.effective_median_weight)?;
    checks::block_weight_check(block_weight, context.median_weight_for_block_reward)?;

    checks::check_amount_txs(block.block.txs.len())?;
    checks::check_prev_id(&block.block, &context.top_hash)?;
    if let Some(median_timestamp) = context.median_block_timestamp {
        // will only be None for the first 60 blocks
        checks::check_timestamp(&block.block, median_timestamp)?;
    }

    checks::check_block_pow(&block.pow_hash, context.next_difficulty)?;

    context
        .current_hard_fork
        .check_block_version_vote(&block.block.header)?;

    Ok(VerifyBlockResponse::MainChain(VerifiedBlockInformation {
        block_hash: block.block_hash,
        block: block.block,
        txs,
        pow_hash: block.pow_hash,
        generated_coins,
        weight: block_weight,
        height: context.chain_height,
        long_term_weight: context.next_block_long_term_weight(block_weight),
        hf_vote: HardFork::V1,
        cumulative_difficulty: context.cumulative_difficulty + context.next_difficulty,
    }))
}

async fn verify_main_chain_block<C, TxV, TxP>(
    block: Block,
    context_svc: C,
    tx_verifier_svc: TxV,
    tx_pool: TxP,
) -> Result<VerifyBlockResponse, ConsensusError>
where
    C: Service<BlockChainContextRequest, Response = BlockChainContext, Error = tower::BoxError>
        + Send
        + 'static,
    C::Future: Send + 'static,
    TxV: Service<VerifyTxRequest, Response = VerifyTxResponse, Error = ConsensusError>,
    TxP: Service<TxPoolRequest, Response = TxPoolResponse, Error = TxNotInPool>
        + Clone
        + Send
        + 'static,
{
    tracing::debug!("getting blockchain context");
    let checked_context = context_svc
        .oneshot(BlockChainContextRequest)
        .await
        .map_err(Into::<ConsensusError>::into)?;

    // TODO: should we unwrap here, we did just get the data so it should be ok.
    let context = checked_context.unchecked_blockchain_context().clone();

    tracing::debug!("got blockchain context: {:?}", context);

    let TxPoolResponse::Transactions(txs) = tx_pool
        .oneshot(TxPoolRequest::Transactions(block.txs.clone()))
        .await?;

    let block_weight = block.miner_tx.weight() + txs.iter().map(|tx| tx.tx_weight).sum::<usize>();
    let total_fees = txs.iter().map(|tx| tx.fee).sum::<u64>();

    tx_verifier_svc
        .oneshot(VerifyTxRequest::Block {
            txs: txs.clone(),
            current_chain_height: context.chain_height,
            time_for_time_lock: context.current_adjusted_timestamp_for_time_lock(),
            hf: context.current_hard_fork,
            re_org_token: context.re_org_token.clone(),
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

    Ok(VerifyBlockResponse::MainChain(VerifiedBlockInformation {
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
    }))
}

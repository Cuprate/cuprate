use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::FutureExt;
use monero_serai::block::Block;
use tower::{Service, ServiceExt};

use crate::{
    context::{BlockChainContext, BlockChainContextRequest},
    transactions::{TransactionVerificationData, VerifyTxRequest, VerifyTxResponse},
    ConsensusError, HardFork, TxNotInPool, TxPoolRequest, TxPoolResponse,
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
    MainChain(Block),
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
        let tx_pool = self.tx_pool.clone();

        async move {
            match req {
                VerifyBlockRequest::MainChain(block) => {
                    verify_main_chain_block(block, context_svc, tx_verifier_svc, tx_pool).await
                }
            }
        }
        .boxed()
    }
}

async fn verify_main_chain_block<C, TxV, TxP>(
    block: Block,
    context_svc: C,
    tx_verifier_svc: TxV,
    tx_pool: TxP,
) -> Result<VerifiedBlockInformation, ConsensusError>
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
    let context = checked_context.blockchain_context().unwrap().clone();

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

use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::FutureExt;
use monero_serai::block::Block;
use monero_serai::transaction::Input;
use tower::{Service, ServiceExt};

use monero_consensus::{
    blocks::{calculate_pow_hash, check_block, check_block_pow, BlockError, RandomX},
    miner_tx::MinerTxError,
    ConsensusError, HardFork,
};

use crate::{
    context::{BlockChainContextRequest, BlockChainContextResponse},
    transactions::{TransactionVerificationData, VerifyTxRequest, VerifyTxResponse},
    ExtendedConsensusError, TxNotInPool, TxPoolRequest, TxPoolResponse,
};

#[derive(Debug)]
pub struct PrePreparedBlockExPOW {
    pub block: Block,
    pub block_blob: Vec<u8>,

    pub hf_vote: HardFork,
    pub hf_version: HardFork,

    pub block_hash: [u8; 32],

    pub miner_tx_weight: usize,
}

impl PrePreparedBlockExPOW {
    pub fn new(block: Block) -> Result<PrePreparedBlockExPOW, ConsensusError> {
        let (hf_version, hf_vote) =
            HardFork::from_block_header(&block.header).map_err(BlockError::HardForkError)?;

        Ok(PrePreparedBlockExPOW {
            block_blob: block.serialize(),
            hf_vote,
            hf_version,

            block_hash: block.hash(),

            miner_tx_weight: block.miner_tx.weight(),
            block,
        })
    }
}

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

impl PrePreparedBlock {
    pub fn new<R: RandomX>(
        block: PrePreparedBlockExPOW,
        randomx_vm: &R,
    ) -> Result<PrePreparedBlock, ConsensusError> {
        let Some(Input::Gen(height)) = block.block.miner_tx.prefix.inputs.first() else {
            Err(ConsensusError::Block(BlockError::MinerTxError(
                MinerTxError::InputNotOfTypeGen,
            )))?
        };

        Ok(PrePreparedBlock {
            block_blob: block.block_blob,
            hf_vote: block.hf_vote,
            hf_version: block.hf_version,

            block_hash: block.block_hash,
            pow_hash: calculate_pow_hash(
                randomx_vm,
                &block.block.serialize_hashable(),
                *height,
                &block.hf_version,
            )?,

            miner_tx_weight: block.block.miner_tx.weight(),
            block: block.block,
        })
    }
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
    MainChainPrepared(PrePreparedBlock),
}

pub enum VerifyBlockResponse {
    MainChain(VerifiedBlockInformation),
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
    C: Service<BlockChainContextRequest, Response = BlockChainContextResponse>
        + Clone
        + Send
        + 'static,
    TxV: Service<VerifyTxRequest, Response = VerifyTxResponse, Error = ExtendedConsensusError>
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

    TxP: Service<TxPoolRequest, Response = TxPoolResponse, Error = TxNotInPool>
        + Clone
        + Send
        + 'static,
    TxP::Future: Send + 'static,
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
        let tx_pool = self.tx_pool.clone();

        async move {
            match req {
                VerifyBlockRequest::MainChain(block) => {
                    verify_main_chain_block(block, context_svc, tx_verifier_svc, tx_pool).await
                }
                VerifyBlockRequest::MainChainPrepared(prepped_block) => {
                    verify_main_chain_block_prepared(
                        prepped_block,
                        context_svc,
                        tx_verifier_svc,
                        tx_pool,
                    )
                    .await
                }
            }
        }
        .boxed()
    }
}

async fn verify_main_chain_block_prepared<C, TxV, TxP>(
    prepped_block: PrePreparedBlock,
    context_svc: C,
    tx_verifier_svc: TxV,
    tx_pool: TxP,
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
    TxP: Service<TxPoolRequest, Response = TxPoolResponse, Error = TxNotInPool>
        + Clone
        + Send
        + 'static,
{
    tracing::debug!("getting blockchain context");
    let BlockChainContextResponse::Context(checked_context) = context_svc
        .oneshot(BlockChainContextRequest::Get)
        .await
        .map_err(Into::<ExtendedConsensusError>::into)?
    else {
        panic!("Context service returned wrong response!");
    };

    let context = checked_context.unchecked_blockchain_context().clone();

    tracing::debug!("got blockchain context: {:?}", context);

    let TxPoolResponse::Transactions(txs) = tx_pool
        .oneshot(TxPoolRequest::Transactions(prepped_block.block.txs.clone()))
        .await?;

    tx_verifier_svc
        .oneshot(VerifyTxRequest::Block {
            txs: txs.clone(),
            current_chain_height: context.chain_height,
            time_for_time_lock: context.current_adjusted_timestamp_for_time_lock(),
            hf: context.current_hf,
            re_org_token: context.re_org_token.clone(),
        })
        .await?;

    let block_weight =
        prepped_block.miner_tx_weight + txs.iter().map(|tx| tx.tx_weight).sum::<usize>();
    let total_fees = txs.iter().map(|tx| tx.fee).sum::<u64>();

    let (hf_vote, generated_coins) = check_block(
        &prepped_block.block,
        total_fees,
        block_weight,
        prepped_block.block_blob.len(),
        &context.context_to_verify_block,
    )
    .map_err(ConsensusError::Block)?;

    check_block_pow(&prepped_block.pow_hash, context.next_difficulty)
        .map_err(ConsensusError::Block)?;

    Ok(VerifyBlockResponse::MainChain(VerifiedBlockInformation {
        block_hash: prepped_block.block_hash,
        block: prepped_block.block,
        txs,
        pow_hash: prepped_block.pow_hash,
        generated_coins,
        weight: block_weight,
        height: context.chain_height,
        long_term_weight: context.next_block_long_term_weight(block_weight),
        hf_vote,
        cumulative_difficulty: context.cumulative_difficulty + context.next_difficulty,
    }))
}

async fn verify_main_chain_block<C, TxV, TxP>(
    _block: Block,
    _context_svc: C,
    _tx_verifier_svc: TxV,
    _tx_pool: TxP,
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
    TxP: Service<TxPoolRequest, Response = TxPoolResponse, Error = TxNotInPool>
        + Clone
        + Send
        + 'static,
{
    todo!("Single main chain block.");

    /*
    tracing::debug!("getting blockchain context");
    let BlockChainContextResponse::Context(checked_context) = context_svc
        .oneshot(BlockChainContextRequest::Get)
        .await
        .map_err(Into::<ExtendedConsensusError>::into)?
    else {
        panic!("Context service returned wrong response!");
    };

    let context = checked_context.unchecked_blockchain_context().clone();

    tracing::debug!("got blockchain context: {:?}", context);

    let TxPoolResponse::Transactions(txs) = tx_pool
        .oneshot(TxPoolRequest::Transactions(block.txs.clone()))
        .await?;

    tx_verifier_svc
        .oneshot(VerifyTxRequest::Block {
            txs: txs.clone(),
            current_chain_height: context.chain_height,
            time_for_time_lock: context.current_adjusted_timestamp_for_time_lock(),
            hf: context.current_hf,
            re_org_token: context.re_org_token.clone(),
        })
        .await?;

    let block_weight = block.miner_tx.weight() + txs.iter().map(|tx| tx.tx_weight).sum::<usize>();
    let total_fees = txs.iter().map(|tx| tx.fee).sum::<u64>();

    let (hf_vote, generated_coins) = check_block(
        &block,
        total_fees,
        block_weight,
        block.serialize().len(),
        &context.context_to_verify_block,
    )
    .map_err(ConsensusError::Block)?;

    let hashing_blob = block.serialize_hashable();

    // do POW test last
    let chain_height = context.chain_height;
    let current_hf = context.current_hf;
    let pow_hash = todo!();
    /*
       rayon_spawn_async(move || calculate_pow_hash(, &hashing_blob, chain_height, &current_hf))
           .await
           .map_err(ConsensusError::Block)?;

    */

    check_block_pow(&pow_hash, context.next_difficulty).map_err(ConsensusError::Block)?;

    Ok(VerifyBlockResponse::MainChain(VerifiedBlockInformation {
        block_hash: block.hash(),
        block,
        txs,
        pow_hash,
        generated_coins,
        weight: block_weight,
        height: context.chain_height,
        long_term_weight: context.next_block_long_term_weight(block_weight),
        hf_vote,
        cumulative_difficulty: context.cumulative_difficulty + context.next_difficulty,
    }))
     */
}

use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use cuprate_helper::asynch::rayon_spawn_async;
use futures::FutureExt;
use monero_serai::block::Block;
use monero_serai::transaction::{Input, Transaction};
use rayon::prelude::*;
use tower::{Service, ServiceExt};

use monero_consensus::blocks::{is_randomx_seed_height, randomx_seed_height};
use monero_consensus::{
    blocks::{calculate_pow_hash, check_block, check_block_pow, BlockError, RandomX},
    miner_tx::MinerTxError,
    ConsensusError, HardFork,
};

use crate::context::rx_seed::RandomXVM;
use crate::transactions::{batch_setup_txs, contextual_data, OutputCache};
use crate::{
    context::{BlockChainContextRequest, BlockChainContextResponse},
    transactions::{TransactionVerificationData, VerifyTxRequest, VerifyTxResponse},
    Database, ExtendedConsensusError, TxNotInPool, TxPoolRequest, TxPoolResponse,
};

#[derive(Debug)]
pub struct PrePreparedBlockExPOW {
    pub block: Block,
    pub block_blob: Vec<u8>,

    pub hf_vote: HardFork,
    pub hf_version: HardFork,

    pub block_hash: [u8; 32],
    pub height: u64,

    pub miner_tx_weight: usize,
}

impl PrePreparedBlockExPOW {
    pub fn new(block: Block) -> Result<PrePreparedBlockExPOW, ConsensusError> {
        let (hf_version, hf_vote) =
            HardFork::from_block_header(&block.header).map_err(BlockError::HardForkError)?;

        let Some(Input::Gen(height)) = block.miner_tx.prefix.inputs.first() else {
            Err(ConsensusError::Block(BlockError::MinerTxError(
                MinerTxError::InputNotOfTypeGen,
            )))?
        };

        Ok(PrePreparedBlockExPOW {
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
    pub fn new(block: Block) -> Result<PrePreparedBlock, ConsensusError> {
        struct DummyRX;

        impl RandomX for DummyRX {
            type Error = ();
            fn calculate_hash(&self, _: &[u8]) -> Result<[u8; 32], Self::Error> {
                panic!("DummyRX cant calculate hash")
            }
        }

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

            pow_hash: calculate_pow_hash::<DummyRX>(
                None,
                &block.serialize_hashable(),
                *height,
                &hf_version,
            )?,
            miner_tx_weight: block.miner_tx.weight(),
            block,
        })
    }

    pub fn new_rx<R: RandomX>(
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
                Some(randomx_vm),
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
    MainChainBatchPrep(Vec<(Block, Vec<Transaction>)>),
    MainChain(Block),
    MainChainPrepared(PrePreparedBlock),
}

pub enum VerifyBlockResponse {
    MainChain(VerifiedBlockInformation),
    MainChainBatchPrep(
        Vec<PrePreparedBlock>,
        Vec<Vec<Arc<TransactionVerificationData>>>,
    ),
}

// TODO: it is probably a bad idea for this to derive clone, if 2 places (RPC, P2P) receive valid but different blocks
// then they will both get approved but only one should go to main chain.
#[derive(Clone)]
pub struct BlockVerifierService<C: Clone, TxV: Clone, TxP: Clone, D> {
    context_svc: C,
    tx_verifier_svc: TxV,
    tx_pool: TxP,
    database: D,
}

impl<C, TxV, TxP, D> BlockVerifierService<C, TxV, TxP, D>
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
    D: Database + Clone + Send + Sync + 'static,
    D::Future: Send + 'static,
{
    pub fn new(
        context_svc: C,
        tx_verifier_svc: TxV,
        tx_pool: TxP,
        database: D,
    ) -> BlockVerifierService<C, TxV, TxP, D> {
        BlockVerifierService {
            context_svc,
            tx_verifier_svc,
            tx_pool,
            database,
        }
    }
}

impl<C, TxV, TxP, D> Service<VerifyBlockRequest> for BlockVerifierService<C, TxV, TxP, D>
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
        let tx_pool = self.tx_pool.clone();
        let database = self.database.clone();

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
                VerifyBlockRequest::MainChainBatchPrep(blocks) => {
                    batch_verify_main_chain_block(blocks, context_svc, database).await
                }
            }
        }
        .boxed()
    }
}

async fn batch_verify_main_chain_block<C, D>(
    blocks: Vec<(Block, Vec<Transaction>)>,
    mut context_svc: C,
    mut database: D,
) -> Result<VerifyBlockResponse, ExtendedConsensusError>
where
    C: Service<
            BlockChainContextRequest,
            Response = BlockChainContextResponse,
            Error = tower::BoxError,
        > + Send
        + 'static,
    C::Future: Send + 'static,
    D: Database + Clone + Send + Sync + 'static,
    D::Future: Send + 'static,
{
    let (blocks, txs): (Vec<_>, Vec<_>) = blocks.into_iter().unzip();

    tracing::debug!("Calculating block hashes.");
    let blocks: Vec<PrePreparedBlockExPOW> = rayon_spawn_async(|| {
        blocks
            .into_iter()
            .map(PrePreparedBlockExPOW::new)
            .collect::<Result<Vec<_>, _>>()
    })
    .await?;

    let mut timestamps_hfs = Vec::with_capacity(blocks.len());
    let mut new_rx_vm = None;

    for window in blocks.windows(2) {
        if window[0].block_hash != window[1].block.header.previous
            || window[0].height != window[1].height + 1
        {
            Err(ConsensusError::Block(BlockError::PreviousIDIncorrect))?;
        }

        if is_randomx_seed_height(window[0].height) {
            new_rx_vm = Some((window[0].height, window[0].block_hash));
        }

        timestamps_hfs.push((window[0].height, window[0].hf_version))
    }

    tracing::debug!("getting blockchain context");
    let BlockChainContextResponse::Context(checked_context) = context_svc
        .ready()
        .await?
        .call(BlockChainContextRequest::GetContext)
        .await
        .map_err(Into::<ExtendedConsensusError>::into)?
    else {
        panic!("Context service returned wrong response!");
    };

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

    if context.chain_height != blocks[0].height {
        Err(ConsensusError::Block(BlockError::MinerTxError(
            MinerTxError::InputsHeightIncorrect,
        )))?;
    }

    if context.top_hash != blocks[0].block.header.previous {
        Err(ConsensusError::Block(BlockError::PreviousIDIncorrect))?;
    }

    let mut rx_vms = context.rx_vms;

    if let Some((new_vm_height, new_vm_seed)) = new_rx_vm {
        let seed_clone = new_vm_seed.clone();
        let new_vm = rayon_spawn_async(move || {
            Arc::new(RandomXVM::new(&seed_clone).expect("RandomX VM gave an error on set up!"))
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

    let blocks = rayon_spawn_async(move || {
        blocks
            .into_par_iter()
            .zip(difficulties)
            .map(|(block, difficultly)| {
                let height = block.height;
                let block = PrePreparedBlock::new_rx(
                    block,
                    rx_vms.get(&randomx_seed_height(height)).unwrap().as_ref(),
                )?;

                check_block_pow(&block.pow_hash, difficultly)?;
                Ok(block)
            })
            .collect::<Result<Vec<_>, ConsensusError>>()
    })
    .await?;

    let txs = batch_setup_txs(
        txs.into_iter()
            .zip(blocks.iter().map(|block| block.hf_version))
            .collect(),
    )
    .await?;

    let mut complete_block_idx = 0;

    let mut out_cache = OutputCache::new();

    out_cache
        .extend_from_block(
            blocks
                .iter()
                .map(|block| &block.block)
                .zip(txs.iter().map(Vec::as_slice)),
            &mut database,
        )
        .await?;

    for (idx, hf) in blocks
        .windows(2)
        .enumerate()
        .filter(|(_, block)| block[0].hf_version != blocks[1].hf_version)
        .map(|(i, block)| (i, &block[0].hf_version))
    {
        contextual_data::batch_fill_ring_member_info(
            txs.iter()
                .take(idx + 1)
                .skip(complete_block_idx)
                .flat_map(|txs| txs.iter()),
            hf,
            context.re_org_token.clone(),
            database.clone(),
            Some(&out_cache),
        )
        .await?;

        complete_block_idx = idx + 1;
    }

    if complete_block_idx != blocks.len() {
        contextual_data::batch_fill_ring_member_info(
            txs.iter()
                .skip(complete_block_idx)
                .flat_map(|txs| txs.iter()),
            &blocks.last().unwrap().hf_version,
            context.re_org_token.clone(),
            database.clone(),
            Some(&out_cache),
        )
        .await?;
    }

    Ok(VerifyBlockResponse::MainChainBatchPrep(blocks, txs))
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
        .oneshot(BlockChainContextRequest::GetContext)
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

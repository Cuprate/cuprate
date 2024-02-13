use std::{
    collections::HashSet,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use cuprate_helper::asynch::rayon_spawn_async;
use futures::FutureExt;
use monero_serai::{
    block::Block,
    transaction::{Input, Transaction},
};
use rayon::prelude::*;
use tower::{Service, ServiceExt};

use monero_consensus::{
    blocks::{
        calculate_pow_hash, check_block, check_block_pow, is_randomx_seed_height,
        randomx_seed_height, BlockError, RandomX,
    },
    miner_tx::MinerTxError,
    ConsensusError, HardFork,
};

use crate::{
    context::{
        rx_vms::RandomXVM, BlockChainContextRequest, BlockChainContextResponse,
        RawBlockChainContext,
    },
    transactions::{
        batch_setup_txs, contextual_data, OutputCache, TransactionVerificationData,
        VerifyTxRequest, VerifyTxResponse,
    },
    Database, ExtendedConsensusError,
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
        randomx_vm: Option<&R>,
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
    MainChainBatchPrep(Vec<(Block, Vec<Transaction>)>),
    MainChain {
        block: Block,
        prepared_txs: Vec<Arc<TransactionVerificationData>>,
        txs: Vec<Transaction>,
    },
    MainChainPrepared(PrePreparedBlock, Vec<Arc<TransactionVerificationData>>),
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
pub struct BlockVerifierService<C: Clone, TxV: Clone, D> {
    context_svc: C,
    tx_verifier_svc: TxV,
    database: D,
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
    pub fn new(
        context_svc: C,
        tx_verifier_svc: TxV,
        database: D,
    ) -> BlockVerifierService<C, TxV, D> {
        BlockVerifierService {
            context_svc,
            tx_verifier_svc,
            database,
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
        let database = self.database.clone();

        async move {
            match req {
                VerifyBlockRequest::MainChain {
                    block,
                    prepared_txs,
                    txs,
                } => {
                    verify_main_chain_block(block, txs, prepared_txs, context_svc, tx_verifier_svc)
                        .await
                }
                VerifyBlockRequest::MainChainPrepared(prepped_block, txs) => {
                    verify_main_chain_block_prepared(
                        prepped_block,
                        txs,
                        context_svc,
                        tx_verifier_svc,
                        None,
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
            || window[0].height != window[1].height - 1
        {
            Err(ConsensusError::Block(BlockError::PreviousIDIncorrect))?;
        }

        if is_randomx_seed_height(window[0].height) {
            new_rx_vm = Some((window[0].height, window[0].block_hash));
        }

        timestamps_hfs.push((window[0].block.header.timestamp, window[0].hf_version))
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

    let blocks = rayon_spawn_async(move || {
        blocks
            .into_par_iter()
            .zip(difficulties)
            .map(|(block, difficultly)| {
                let height = block.height;
                let block = PrePreparedBlock::new_rx(
                    block,
                    rx_vms.get(&randomx_seed_height(height)).map(AsRef::as_ref),
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

async fn verify_main_chain_block_prepared<C, TxV>(
    prepped_block: PrePreparedBlock,
    txs: Vec<Arc<TransactionVerificationData>>,
    context_svc: C,
    tx_verifier_svc: TxV,
    context: Option<RawBlockChainContext>,
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
    let context = match context {
        Some(context) => context,
        None => {
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
            context
        }
    };

    check_block_pow(&prepped_block.pow_hash, context.next_difficulty)
        .map_err(ConsensusError::Block)?;

    // Check that the txs included are what we need and that there are not any extra.
    // Collecting into a HashSet could hide duplicates but we check Key Images are unique so someone would have to find
    // a hash collision to include duplicate txs here.
    let mut tx_hashes = txs.iter().map(|tx| &tx.tx_hash).collect::<HashSet<_>>();
    for tx_hash in &prepped_block.block.txs {
        if !tx_hashes.remove(tx_hash) {
            return Err(ExtendedConsensusError::TxsIncludedWithBlockIncorrect);
        }
    }
    if !tx_hashes.is_empty() {
        return Err(ExtendedConsensusError::TxsIncludedWithBlockIncorrect);
    }

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

async fn verify_main_chain_block<C, TxV>(
    block: Block,
    txs: Vec<Transaction>,
    mut prepared_txs: Vec<Arc<TransactionVerificationData>>,
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

    let context = checked_context.unchecked_blockchain_context().clone();
    tracing::debug!("got blockchain context: {:?}", context);

    let rx_vms = context.rx_vms.clone();
    let prepped_block = rayon_spawn_async(move || {
        let prepped_block_ex_pow = PrePreparedBlockExPOW::new(block)?;
        let height = prepped_block_ex_pow.height;

        PrePreparedBlock::new_rx(prepped_block_ex_pow, rx_vms.get(&height).map(AsRef::as_ref))
    })
    .await?;

    check_block_pow(&prepped_block.pow_hash, context.cumulative_difficulty)
        .map_err(ConsensusError::Block)?;

    prepared_txs.append(&mut batch_setup_txs(vec![(txs, context.current_hf)]).await?[0]);

    verify_main_chain_block_prepared(
        prepped_block,
        prepared_txs,
        context_svc,
        tx_verifier_svc,
        Some(context),
    )
    .await
}

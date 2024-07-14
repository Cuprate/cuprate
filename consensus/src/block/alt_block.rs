use crate::block::PreparedBlock;
use crate::context::difficulty::DifficultyCache;
use crate::context::rx_vms::RandomXVM;
use crate::context::weight::BlockWeightsCache;
use crate::context::{weight, AltChainContextCache, AltChainRequestToken};
use crate::transactions::TransactionVerificationData;
use crate::{
    BlockChainContextRequest, BlockChainContextResponse, ExtendedConsensusError,
    VerifyBlockResponse, VerifyTxRequest,
};
use cuprate_consensus_rules::blocks::{
    check_block, check_block_pow, check_block_weight, randomx_seed_height, BlockError,
};
use cuprate_consensus_rules::miner_tx::MinerTxError;
use cuprate_consensus_rules::ConsensusError;
use cuprate_helper::asynch::rayon_spawn_async;
use cuprate_types::{
    AltBlockInformation, Chain, ChainID, VerifiedBlockInformation, VerifiedTransactionInformation,
};
use monero_serai::block::Block;
use monero_serai::transaction::Input;
use std::collections::HashMap;
use std::sync::Arc;
use tower::{Service, ServiceExt};

pub async fn sanity_check_alt_block<C>(
    block: Block,
    mut txs: HashMap<[u8; 32], TransactionVerificationData>,
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
    let BlockChainContextResponse::AltChainContextCache(mut alt_context_cache) = context_svc
        .ready()
        .await?
        .call(BlockChainContextRequest::AltChainContextCache {
            prev_id: block.header.previous,
            _token: AltChainRequestToken,
        })
        .await?
    else {
        panic!("Context service returned wrong response!");
    };

    let [Input::Gen(height)] = &block.miner_tx.prefix.inputs[..] else {
        Err(ConsensusError::Block(BlockError::MinerTxError(
            MinerTxError::InputNotOfTypeGen,
        )))?
    };

    if *height != alt_context_cache.chain_height {
        Err(ConsensusError::Block(BlockError::MinerTxError(
            MinerTxError::InputsHeightIncorrect,
        )))?
    }

    let prepped_block = {
        let rx_vm = alt_rx_vm(
            alt_context_cache.chain_height,
            block.header.major_version,
            alt_context_cache.parent_chain,
            &mut alt_context_cache,
            &mut context_svc,
        )
        .await?;

        rayon_spawn_async(move || PreparedBlock::new(block, rx_vm.as_deref())).await?
    };

    let difficulty_cache = alt_difficulty_cache(
        prepped_block.block.header.previous,
        &mut alt_context_cache,
        &mut context_svc,
    )
    .await?;

    let next_difficulty = difficulty_cache.next_difficulty(&prepped_block.hf_version);

    check_block_pow(&prepped_block.pow_hash, next_difficulty).map_err(ConsensusError::Block)?;

    let cumulative_difficulty = difficulty_cache.cumulative_difficulty() + next_difficulty;

    if prepped_block.block.txs.len() != txs.len() {
        return Err(ExtendedConsensusError::TxsIncludedWithBlockIncorrect);
    }

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

    let block_weight =
        prepped_block.miner_tx_weight + ordered_txs.iter().map(|tx| tx.tx_weight).sum::<usize>();

    let alt_weight_cache = alt_weight_cache(
        prepped_block.block.header.previous,
        &mut alt_context_cache,
        &mut context_svc,
    )
    .await?;

    check_block_weight(
        block_weight,
        alt_weight_cache.median_for_block_reward(&prepped_block.hf_version),
    )
    .map_err(ConsensusError::Block)?;

    let long_term_weight = weight::calculate_block_long_term_weight(
        &prepped_block.hf_version,
        block_weight,
        alt_weight_cache.median_long_term_weight(),
    );

    let chain_id = *alt_context_cache
        .chain_id
        .get_or_insert_with(|| ChainID(rand::random()));

    Ok(VerifyBlockResponse::AltChain(AltBlockInformation {
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
        weight: block_weight,
        height: alt_context_cache.chain_height,
        long_term_weight,
        cumulative_difficulty,
        chain_id,
    }))
}

async fn alt_rx_vm<C>(
    block_height: u64,
    hf: u8,
    parent_chain: Chain,
    alt_chain_context: &mut AltChainContextCache,
    context_svc: C,
) -> Result<Option<Arc<RandomXVM>>, ExtendedConsensusError>
where
    C: Service<
            BlockChainContextRequest,
            Response = BlockChainContextResponse,
            Error = tower::BoxError,
        > + Send,
    C::Future: Send + 'static,
{
    if hf < 12 {
        return Ok(None);
    }

    let seed_height = randomx_seed_height(block_height);

    let cached_vm = match alt_chain_context
        .cached_rx_vm
        .take()
        .filter(|(cached_seed_height, _)| seed_height == *cached_seed_height)
    {
        Some((cached_seed_height, vm)) if seed_height == cached_seed_height => {
            (cached_seed_height, vm)
        }
        _ => {
            let BlockChainContextResponse::AltChainRxVM(vm) = context_svc
                .oneshot(BlockChainContextRequest::AltChainRxVM {
                    height: block_height,
                    chain: parent_chain,
                    _token: AltChainRequestToken,
                })
                .await?
            else {
                panic!("Context service returned wrong response!");
            };

            (seed_height, vm)
        }
    };

    Ok(Some(
        alt_chain_context.cached_rx_vm.insert(cached_vm).1.clone(),
    ))
}

async fn alt_difficulty_cache<C>(
    prev_id: [u8; 32],
    alt_chain_context: &mut AltChainContextCache,
    context_svc: C,
) -> Result<&mut DifficultyCache, ExtendedConsensusError>
where
    C: Service<
            BlockChainContextRequest,
            Response = BlockChainContextResponse,
            Error = tower::BoxError,
        > + Send,
    C::Future: Send + 'static,
{
    match &mut alt_chain_context.difficulty_cache {
        Some(cache) => Ok(cache),
        difficulty_cache => {
            let BlockChainContextResponse::AltChainDifficultyCache(cache) = context_svc
                .oneshot(BlockChainContextRequest::AltChainDifficultyCache {
                    prev_id,
                    _token: AltChainRequestToken,
                })
                .await?
            else {
                panic!("Context service returned wrong response!");
            };

            Ok(difficulty_cache.insert(cache))
        }
    }
}

async fn alt_weight_cache<C>(
    prev_id: [u8; 32],
    alt_chain_context: &mut AltChainContextCache,
    context_svc: C,
) -> Result<&mut BlockWeightsCache, ExtendedConsensusError>
where
    C: Service<
            BlockChainContextRequest,
            Response = BlockChainContextResponse,
            Error = tower::BoxError,
        > + Send,
    C::Future: Send + 'static,
{
    match &mut alt_chain_context.weight_cache {
        Some(cache) => Ok(cache),
        weight_cache => {
            let BlockChainContextResponse::AltChainWeightCache(cache) = context_svc
                .oneshot(BlockChainContextRequest::AltChainWeightCache {
                    prev_id,
                    _token: AltChainRequestToken,
                })
                .await?
            else {
                panic!("Context service returned wrong response!");
            };

            Ok(weight_cache.insert(cache))
        }
    }
}

//! Alt Blocks
//!
//! Alt blocks are sanity checked by [`sanity_check_alt_block`], that function will also compute the cumulative
//! difficulty of the alt chain so callers will know if they should re-org to the alt chain.
use std::{collections::HashMap, sync::Arc};

use monero_serai::{block::Block, transaction::Input};
use tower::{Service, ServiceExt};

use cuprate_consensus_rules::{
    blocks::{
        check_block_pow, check_block_weight, check_timestamp, randomx_seed_height, BlockError,
    },
    miner_tx::MinerTxError,
    ConsensusError,
};
use cuprate_helper::{asynch::rayon_spawn_async, cast::u64_to_usize};
use cuprate_types::{
    AltBlockInformation, Chain, ChainId, TransactionVerificationData,
    VerifiedTransactionInformation,
};

use crate::{
    block::{free::pull_ordered_transactions, PreparedBlock},
    context::{
        difficulty::DifficultyCache,
        rx_vms::RandomXVM,
        weight::{self, BlockWeightsCache},
        AltChainContextCache, AltChainRequestToken, BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW,
    },
    BlockChainContextRequest, BlockChainContextResponse, ExtendedConsensusError,
    VerifyBlockResponse,
};

/// This function sanity checks an alt-block.
///
/// Returns [`AltBlockInformation`], which contains the cumulative difficulty of the alt chain.
///
/// This function only checks the block's PoW and its weight.
pub async fn sanity_check_alt_block<C>(
    block: Block,
    txs: HashMap<[u8; 32], TransactionVerificationData>,
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
    // Fetch the alt-chains context cache.
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

    // Check if the block's miner input is formed correctly.
    let [Input::Gen(height)] = &block.miner_transaction.prefix().inputs[..] else {
        Err(ConsensusError::Block(BlockError::MinerTxError(
            MinerTxError::InputNotOfTypeGen,
        )))?
    };

    if *height != alt_context_cache.chain_height {
        Err(ConsensusError::Block(BlockError::MinerTxError(
            MinerTxError::InputsHeightIncorrect,
        )))?
    }

    // prep the alt block.
    let prepped_block = {
        let rx_vm = alt_rx_vm(
            alt_context_cache.chain_height,
            block.header.hardfork_version,
            alt_context_cache.parent_chain,
            &mut alt_context_cache,
            &mut context_svc,
        )
        .await?;

        rayon_spawn_async(move || PreparedBlock::new(block, rx_vm.as_deref())).await?
    };

    // get the difficulty cache for this alt chain.
    let difficulty_cache = alt_difficulty_cache(
        prepped_block.block.header.previous,
        &mut alt_context_cache,
        &mut context_svc,
    )
    .await?;

    // Check the alt block timestamp is in the correct range.
    if let Some(median_timestamp) =
        difficulty_cache.median_timestamp(u64_to_usize(BLOCKCHAIN_TIMESTAMP_CHECK_WINDOW))
    {
        check_timestamp(&prepped_block.block, median_timestamp).map_err(ConsensusError::Block)?
    };

    let next_difficulty = difficulty_cache.next_difficulty(&prepped_block.hf_version);
    // make sure the block's PoW is valid for this difficulty.
    check_block_pow(&prepped_block.pow_hash, next_difficulty).map_err(ConsensusError::Block)?;

    let cumulative_difficulty = difficulty_cache.cumulative_difficulty() + next_difficulty;

    let ordered_txs = pull_ordered_transactions(&prepped_block.block, txs)?;

    let block_weight =
        prepped_block.miner_tx_weight + ordered_txs.iter().map(|tx| tx.tx_weight).sum::<usize>();

    let alt_weight_cache = alt_weight_cache(
        prepped_block.block.header.previous,
        &mut alt_context_cache,
        &mut context_svc,
    )
    .await?;

    // Check the block weight is below the limit.
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

    // Get the chainID or generate a new one if this is the first alt block in this alt chain.
    let chain_id = *alt_context_cache
        .chain_id
        .get_or_insert_with(|| ChainId(rand::random()));

    // Create the alt block info.
    let block_info = AltBlockInformation {
        block_hash: prepped_block.block_hash,
        block: prepped_block.block,
        block_blob: prepped_block.block_blob,
        txs: ordered_txs
            .into_iter()
            .map(|tx| VerifiedTransactionInformation {
                tx_blob: tx.tx_blob,
                tx_weight: tx.tx_weight,
                fee: tx.fee,
                tx_hash: tx.tx_hash,
                tx: tx.tx,
            })
            .collect(),
        pow_hash: prepped_block.pow_hash,
        weight: block_weight,
        height: alt_context_cache.chain_height,
        long_term_weight,
        cumulative_difficulty,
        chain_id,
    };

    // Add this block to the cache.
    alt_context_cache.add_new_block(
        block_info.height,
        block_info.block_hash,
        block_info.weight,
        block_info.long_term_weight,
        block_info.block.header.timestamp,
    );

    // Add this alt cache back to the context service.
    context_svc
        .oneshot(BlockChainContextRequest::AddAltChainContextCache {
            prev_id: block_info.block.header.previous,
            cache: alt_context_cache,
            _token: AltChainRequestToken,
        })
        .await?;

    Ok(VerifyBlockResponse::AltChain(block_info))
}

/// Retrieves the alt RX VM for the chosen block height.
///
/// If the `hf` is less than 12 (the height RX activates), then [`None`] is returned.
async fn alt_rx_vm<C>(
    block_height: usize,
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

    let cached_vm = match alt_chain_context.cached_rx_vm.take() {
        // If the VM is cached and the height is the height we need, we can use this VM.
        Some((cached_seed_height, vm)) if seed_height == cached_seed_height => {
            (cached_seed_height, vm)
        }
        // Otherwise we need to make a new VM.
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

/// Returns the [`DifficultyCache`] for the alt chain.
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
    // First look to see if the difficulty cache for this alt chain is already cached.
    match &mut alt_chain_context.difficulty_cache {
        Some(cache) => Ok(cache),
        // Otherwise make a new one.
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

/// Returns the [`BlockWeightsCache`] for the alt chain.
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
    // First look to see if the weight cache for this alt chain is already cached.
    match &mut alt_chain_context.weight_cache {
        Some(cache) => Ok(cache),
        // Otherwise make a new one.
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

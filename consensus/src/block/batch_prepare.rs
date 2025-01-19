use std::{collections::HashMap, sync::Arc};

use crate::batch_verifier::MultiThreadedBatchVerifier;
use crate::block::free::order_transactions;
use crate::transactions::PrepTransactionsState;
use crate::{
    block::{free::pull_ordered_transactions, PreparedBlock, PreparedBlockExPow},
    transactions::new_tx_verification_data,
    BlockChainContextRequest, BlockChainContextResponse, ExtendedConsensusError,
};
use cuprate_consensus_context::rx_vms::RandomXVm;
use cuprate_consensus_context::BlockchainContextService;
use cuprate_consensus_rules::{
    blocks::{check_block_pow, is_randomx_seed_height, randomx_seed_height, BlockError},
    hard_forks::HardForkError,
    miner_tx::MinerTxError,
    ConsensusError, HardFork,
};
use cuprate_helper::asynch::rayon_spawn_async;
use cuprate_types::TransactionVerificationData;
use monero_serai::{block::Block, transaction::Transaction};
use rayon::prelude::*;
use tower::{Service, ServiceExt};
use tracing::instrument;

/// Batch prepares a list of blocks for verification.
#[instrument(level = "debug", name = "batch_prep_blocks", skip_all, fields(amt = blocks.len()))]
pub async fn batch_prepare_main_chain_blocks(
    blocks: Vec<(Block, Vec<Transaction>)>,
    context_svc: &mut BlockchainContextService,
) -> Result<Vec<(PreparedBlock, Vec<TransactionVerificationData>)>, ExtendedConsensusError> {
    let (blocks, txs): (Vec<_>, Vec<_>) = blocks.into_iter().unzip();

    tracing::debug!("Calculating block hashes.");
    let blocks: Vec<PreparedBlockExPow> = rayon_spawn_async(|| {
        blocks
            .into_iter()
            .map(PreparedBlockExPow::new)
            .collect::<Result<Vec<_>, _>>()
    })
    .await?;

    let Some(last_block) = blocks.last() else {
        return Err(ExtendedConsensusError::NoBlocksToVerify);
    };

    // hard-forks cannot be reversed, so the last block will contain the highest hard fork (provided the
    // batch is valid).
    let top_hf_in_batch = last_block.hf_version;

    // A Vec of (timestamp, HF) for each block to calculate the expected difficulty for each block.
    let mut timestamps_hfs = Vec::with_capacity(blocks.len());
    let mut new_rx_vm = None;

    tracing::debug!("Checking blocks follow each other.");

    // For every block make sure they have the correct height and previous ID
    for window in blocks.windows(2) {
        let block_0 = &window[0];
        let block_1 = &window[1];

        // Make sure no blocks in the batch have a higher hard fork than the last block.
        if block_0.hf_version > top_hf_in_batch {
            return Err(ConsensusError::Block(BlockError::HardForkError(
                HardForkError::VersionIncorrect,
            ))
            .into());
        }

        if block_0.block_hash != block_1.block.header.previous
            || block_0.height != block_1.height - 1
        {
            tracing::debug!("Blocks do not follow each other, verification failed.");
            return Err(ConsensusError::Block(BlockError::PreviousIDIncorrect).into());
        }

        // Cache any potential RX VM seeds as we may need them for future blocks in the batch.
        if is_randomx_seed_height(block_0.height) && top_hf_in_batch >= HardFork::V12 {
            new_rx_vm = Some((block_0.height, block_0.block_hash));
        }

        timestamps_hfs.push((block_0.block.header.timestamp, block_0.hf_version));
    }

    // Calculate the expected difficulties for each block in the batch.
    let BlockChainContextResponse::BatchDifficulties(difficulties) = context_svc
        .ready()
        .await?
        .call(BlockChainContextRequest::BatchGetDifficulties(
            timestamps_hfs,
        ))
        .await?
    else {
        panic!("Context service returned wrong response!");
    };

    // Get the current blockchain context.
    let context = context_svc.blockchain_context();

    // Make sure the blocks follow the main chain.

    if context.chain_height != blocks[0].height {
        tracing::debug!("Blocks do not follow main chain, verification failed.");

        return Err(ConsensusError::Block(BlockError::MinerTxError(
            MinerTxError::InputsHeightIncorrect,
        ))
        .into());
    }

    if context.top_hash != blocks[0].block.header.previous {
        tracing::debug!("Blocks do not follow main chain, verification failed.");

        return Err(ConsensusError::Block(BlockError::PreviousIDIncorrect).into());
    }

    let mut rx_vms = if top_hf_in_batch < HardFork::V12 {
        HashMap::new()
    } else {
        let BlockChainContextResponse::RxVms(rx_vms) = context_svc
            .ready()
            .await?
            .call(BlockChainContextRequest::CurrentRxVms)
            .await?
        else {
            panic!("Blockchain context service returned wrong response!");
        };

        rx_vms
    };

    // If we have a RX seed in the batch calculate it.
    if let Some((new_vm_height, new_vm_seed)) = new_rx_vm {
        tracing::debug!("New randomX seed in batch, initialising VM");

        let new_vm = rayon_spawn_async(move || {
            Arc::new(RandomXVm::new(&new_vm_seed).expect("RandomX VM gave an error on set up!"))
        })
        .await;

        // Give the new VM to the context service, so it can cache it.
        context_svc
            .oneshot(BlockChainContextRequest::NewRXVM((
                new_vm_seed,
                Arc::clone(&new_vm),
            )))
            .await?;

        rx_vms.insert(new_vm_height, new_vm);
    }

    tracing::debug!("Calculating PoW and prepping transaction");

    let blocks = rayon_spawn_async(move || {
        let batch_verifier = MultiThreadedBatchVerifier::new(rayon::current_num_threads());

        let res = blocks
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

                let mut txs = PrepTransactionsState::new()
                    .append_txs(txs)
                    .prepare()?
                    .just_semantic(block.hf_version)
                    .queue(&batch_verifier)?;

                // Order the txs correctly.
                order_transactions(&block.block, &mut txs)?;

                Ok((block, txs))
            })
            .collect::<Result<Vec<_>, ExtendedConsensusError>>()?;

        if !batch_verifier.verify() {
            return Err(ExtendedConsensusError::OneOrMoreBatchVerificationStatementsInvalid);
        }

        Ok(res)
    })
    .await?;

    Ok(blocks)
}

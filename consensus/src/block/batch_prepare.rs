use std::{collections::HashMap, sync::Arc};

use monero_serai::{block::Block, transaction::Transaction};
use rayon::prelude::*;
use tower::{Service, ServiceExt};
use tracing::instrument;

use cuprate_consensus_rules::{
    blocks::{check_block_pow, is_randomx_seed_height, randomx_seed_height, BlockError},
    hard_forks::HardForkError,
    miner_tx::MinerTxError,
    ConsensusError, HardFork,
};
use cuprate_helper::asynch::rayon_spawn_async;

use crate::{
    block::{free::pull_ordered_transactions, PreparedBlock, PreparedBlockExPow},
    context::rx_vms::RandomXVM,
    transactions::new_tx_verification_data,
    BlockChainContextRequest, BlockChainContextResponse, ExtendedConsensusError,
    VerifyBlockResponse,
};

/// Batch prepares a list of blocks for verification.
#[instrument(level = "debug", name = "batch_prep_blocks", skip_all, fields(amt = blocks.len()))]
pub(crate) async fn batch_prepare_main_chain_block<C>(
    blocks: Vec<(Block, Vec<Transaction>)>,
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
            Err(ConsensusError::Block(BlockError::HardForkError(
                HardForkError::VersionIncorrect,
            )))?;
        }

        if block_0.block_hash != block_1.block.header.previous
            || block_0.height != block_1.height - 1
        {
            tracing::debug!("Blocks do not follow each other, verification failed.");
            Err(ConsensusError::Block(BlockError::PreviousIDIncorrect))?;
        }

        // Cache any potential RX VM seeds as we may need them for future blocks in the batch.
        if is_randomx_seed_height(block_0.height) && top_hf_in_batch >= HardFork::V12 {
            new_rx_vm = Some((block_0.height, block_0.block_hash));
        }

        timestamps_hfs.push((block_0.block.header.timestamp, block_0.hf_version))
    }

    // Get the current blockchain context.
    let BlockChainContextResponse::Context(checked_context) = context_svc
        .ready()
        .await?
        .call(BlockChainContextRequest::GetContext)
        .await?
    else {
        panic!("Context service returned wrong response!");
    };

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

    let context = checked_context.unchecked_blockchain_context().clone();

    // Make sure the blocks follow the main chain.

    if context.chain_height != blocks[0].height {
        tracing::debug!("Blocks do not follow main chain, verification failed.");

        Err(ConsensusError::Block(BlockError::MinerTxError(
            MinerTxError::InputsHeightIncorrect,
        )))?;
    }

    if context.top_hash != blocks[0].block.header.previous {
        tracing::debug!("Blocks do not follow main chain, verification failed.");

        Err(ConsensusError::Block(BlockError::PreviousIDIncorrect))?;
    }

    let mut rx_vms = if top_hf_in_batch < HardFork::V12 {
        HashMap::new()
    } else {
        let BlockChainContextResponse::RxVms(rx_vms) = context_svc
            .ready()
            .await?
            .call(BlockChainContextRequest::GetCurrentRxVm)
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
            Arc::new(RandomXVM::new(&new_vm_seed).expect("RandomX VM gave an error on set up!"))
        })
        .await;

        // Give the new VM to the context service, so it can cache it.
        context_svc
            .oneshot(BlockChainContextRequest::NewRXVM((
                new_vm_seed,
                new_vm.clone(),
            )))
            .await?;

        rx_vms.insert(new_vm_height, new_vm);
    }

    tracing::debug!("Calculating PoW and prepping transaction");

    let blocks = rayon_spawn_async(move || {
        blocks
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

                // Now setup the txs.
                let txs = txs
                    .into_par_iter()
                    .map(|tx| {
                        let tx = new_tx_verification_data(tx)?;
                        Ok::<_, ConsensusError>((tx.tx_hash, tx))
                    })
                    .collect::<Result<HashMap<_, _>, _>>()?;

                // Order the txs correctly.
                // TODO: Remove the Arc here
                let ordered_txs = pull_ordered_transactions(&block.block, txs)?
                    .into_iter()
                    .map(Arc::new)
                    .collect();

                Ok((block, ordered_txs))
            })
            .collect::<Result<Vec<_>, ExtendedConsensusError>>()
    })
    .await?;

    Ok(VerifyBlockResponse::MainChainBatchPrepped(blocks))
}

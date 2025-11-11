//! Block Verification.
//!
//! This module contains functions for verifying blocks:
//! - [`verify_main_chain_block`]
//! - [`batch_prepare_main_chain_blocks`]
//! - [`verify_prepped_main_chain_block`]
//! - [`sanity_check_alt_block`]
//!
use std::{collections::HashMap, mem};

use monero_oxide::{block::Block, transaction::Input};
use tower::{Service, ServiceExt};

use cuprate_consensus_context::{
    BlockChainContextRequest, BlockChainContextResponse, BlockchainContextService,
};
use cuprate_helper::asynch::rayon_spawn_async;
use cuprate_types::{
    AltBlockInformation, TransactionVerificationData, VerifiedBlockInformation,
    VerifiedTransactionInformation,
};

use cuprate_consensus_rules::{
    blocks::{
        calculate_pow_hash, check_block, check_block_pow, randomx_seed_height, BlockError, RandomX,
    },
    hard_forks::HardForkError,
    miner_tx::MinerTxError,
    ConsensusError, HardFork,
};

use crate::{transactions::start_tx_verification, Database, ExtendedConsensusError};

mod alt_block;
mod batch_prepare;
mod free;

pub use alt_block::sanity_check_alt_block;
pub use batch_prepare::{batch_prepare_main_chain_blocks, BatchPrepareCache};
use free::pull_ordered_transactions;

/// A pre-prepared block with all data needed to verify it, except the block's proof of work.
#[derive(Debug)]
pub struct PreparedBlockExPow {
    /// The block.
    pub block: Block,
    /// The serialised block's bytes.
    pub block_blob: Vec<u8>,

    /// The block's hard-fork vote.
    pub hf_vote: HardFork,
    /// The block's hard-fork version.
    pub hf_version: HardFork,

    /// The block's hash.
    pub block_hash: [u8; 32],
    /// The height of the block.
    pub height: usize,

    /// The weight of the block's miner transaction.
    pub miner_tx_weight: usize,
}

impl PreparedBlockExPow {
    /// Prepare a new block.
    ///
    /// # Errors
    /// This errors if either the `block`'s:
    /// - Hard-fork values are invalid
    /// - Miner transaction is missing a miner input
    pub fn new(block: Block) -> Result<Self, ConsensusError> {
        let (hf_version, hf_vote) = HardFork::from_block_header(&block.header)
            .map_err(|_| BlockError::HardForkError(HardForkError::HardForkUnknown))?;

        let Some(Input::Gen(height)) = block.miner_transaction().prefix().inputs.first() else {
            return Err(ConsensusError::Block(BlockError::MinerTxError(
                MinerTxError::InputNotOfTypeGen,
            )));
        };

        Ok(Self {
            block_blob: block.serialize(),
            hf_vote,
            hf_version,

            block_hash: block.hash(),
            height: *height,

            miner_tx_weight: block.miner_transaction().weight(),
            block,
        })
    }
}

/// A pre-prepared block with all data needed to verify it.
#[derive(Debug)]
pub struct PreparedBlock {
    /// The block
    pub block: Block,
    /// The serialised blocks bytes
    pub block_blob: Vec<u8>,

    /// The blocks hf vote
    pub hf_vote: HardFork,
    /// The blocks hf version
    pub hf_version: HardFork,

    /// The blocks hash
    pub block_hash: [u8; 32],
    /// The blocks POW hash.
    pub pow_hash: [u8; 32],

    /// The weight of the blocks miner transaction.
    pub miner_tx_weight: usize,
}

impl PreparedBlock {
    /// Creates a new [`PreparedBlock`].
    ///
    /// The randomX VM must be Some if RX is needed or this will panic.
    /// The randomX VM must also be initialised with the correct seed.
    pub fn new<R: RandomX>(block: Block, randomx_vm: Option<&R>) -> Result<Self, ConsensusError> {
        let (hf_version, hf_vote) = HardFork::from_block_header(&block.header)
            .map_err(|_| BlockError::HardForkError(HardForkError::HardForkUnknown))?;

        let [Input::Gen(height)] = &block.miner_transaction().prefix().inputs[..] else {
            return Err(ConsensusError::Block(BlockError::MinerTxError(
                MinerTxError::InputNotOfTypeGen,
            )));
        };

        Ok(Self {
            block_blob: block.serialize(),
            hf_vote,
            hf_version,

            block_hash: block.hash(),
            pow_hash: calculate_pow_hash(
                randomx_vm,
                &block.serialize_pow_hash(),
                *height,
                &hf_version,
            )?,

            miner_tx_weight: block.miner_transaction().weight(),
            block,
        })
    }

    /// Creates a new [`PreparedBlock`] from a [`PreparedBlockExPow`].
    ///
    /// This function will give an invalid proof-of-work hash if `randomx_vm` is not initialised
    /// with the correct seed.
    ///
    /// # Panics
    /// This function will panic if `randomx_vm` is
    /// [`None`] even though RandomX is needed.
    fn new_prepped<R: RandomX>(
        block: PreparedBlockExPow,
        randomx_vm: Option<&R>,
    ) -> Result<Self, ConsensusError> {
        Ok(Self {
            block_blob: block.block_blob,
            hf_vote: block.hf_vote,
            hf_version: block.hf_version,

            block_hash: block.block_hash,
            pow_hash: calculate_pow_hash(
                randomx_vm,
                &block.block.serialize_pow_hash(),
                block.height,
                &block.hf_version,
            )?,

            miner_tx_weight: block.block.miner_transaction().weight(),
            block: block.block,
        })
    }

    /// Creates a new [`PreparedBlock`] from an [`AltBlockInformation`].
    pub fn new_alt_block(block: AltBlockInformation) -> Result<Self, ConsensusError> {
        Ok(Self {
            block_blob: block.block_blob,
            hf_vote: HardFork::from_version(block.block.header.hardfork_version)
                .map_err(|_| BlockError::HardForkError(HardForkError::HardForkUnknown))?,
            hf_version: HardFork::from_vote(block.block.header.hardfork_signal),
            block_hash: block.block_hash,
            pow_hash: block.pow_hash,
            miner_tx_weight: block.block.miner_transaction().weight(),
            block: block.block,
        })
    }
}

/// Fully verify a block and all its transactions.
pub async fn verify_main_chain_block<D>(
    block: Block,
    txs: HashMap<[u8; 32], TransactionVerificationData>,
    context_svc: &mut BlockchainContextService,
    database: D,
) -> Result<VerifiedBlockInformation, ExtendedConsensusError>
where
    D: Database + Clone + Send + 'static,
{
    let context = context_svc.blockchain_context().clone();
    tracing::debug!("got blockchain context: {:?}", context);

    tracing::debug!(
        "Preparing block for verification, expected height: {}",
        context.chain_height
    );

    // Set up the block and just pass it to [`verify_prepped_main_chain_block`]

    // We just use the raw `hardfork_version` here, no need to turn it into a `HardFork`.
    let rx_vms = if block.header.hardfork_version < 12 {
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

    let height = context.chain_height;
    let prepped_block = rayon_spawn_async(move || {
        PreparedBlock::new(
            block,
            rx_vms.get(&randomx_seed_height(height)).map(AsRef::as_ref),
        )
    })
    .await?;

    check_block_pow(&prepped_block.pow_hash, context.next_difficulty)
        .map_err(ConsensusError::Block)?;

    // Check that the txs included are what we need and that there are not any extra.
    let ordered_txs = pull_ordered_transactions(&prepped_block.block, txs)?;

    verify_prepped_main_chain_block(prepped_block, ordered_txs, context_svc, database, None).await
}

/// Fully verify a block that has already been prepared using [`batch_prepare_main_chain_blocks`].
pub async fn verify_prepped_main_chain_block<D>(
    prepped_block: PreparedBlock,
    mut txs: Vec<TransactionVerificationData>,
    context_svc: &mut BlockchainContextService,
    database: D,
    batch_prep_cache: Option<&mut BatchPrepareCache>,
) -> Result<VerifiedBlockInformation, ExtendedConsensusError>
where
    D: Database + Clone + Send + 'static,
{
    let context = context_svc.blockchain_context();

    tracing::debug!("verifying block: {}", hex::encode(prepped_block.block_hash));

    check_block_pow(&prepped_block.pow_hash, context.next_difficulty)
        .map_err(ConsensusError::Block)?;

    if prepped_block.block.transactions.len() != txs.len() {
        return Err(ExtendedConsensusError::TxsIncludedWithBlockIncorrect);
    }

    if !prepped_block.block.transactions.is_empty() {
        for (expected_tx_hash, tx) in prepped_block.block.transactions.iter().zip(txs.iter()) {
            if expected_tx_hash != &tx.tx_hash {
                return Err(ExtendedConsensusError::TxsIncludedWithBlockIncorrect);
            }
        }

        let temp = start_tx_verification()
            .append_prepped_txs(mem::take(&mut txs))
            .prepare()?
            .full(
                context.chain_height,
                context.top_hash,
                context.current_adjusted_timestamp_for_time_lock(),
                context.current_hf,
                database,
                batch_prep_cache.as_deref(),
            )
            .verify()
            .await?;

        txs = temp;
    }

    let block_weight =
        prepped_block.miner_tx_weight + txs.iter().map(|tx| tx.tx_weight).sum::<usize>();
    let total_fees = txs.iter().map(|tx| tx.fee).sum::<u64>();

    tracing::debug!("Verifying block header.");
    let (_, generated_coins) = check_block(
        &prepped_block.block,
        total_fees,
        block_weight,
        prepped_block.block_blob.len(),
        &context.context_to_verify_block,
    )
    .map_err(ConsensusError::Block)?;

    let block = VerifiedBlockInformation {
        block_hash: prepped_block.block_hash,
        block: prepped_block.block,
        block_blob: prepped_block.block_blob,
        txs: txs
            .into_iter()
            .map(|tx| {
                let tx_weight = tx.tx_weight;
                let fee = tx.fee;
                let tx_hash = tx.tx_hash;
                let (tx, tx_prunable_blob) = tx.tx.pruned_with_prunable();
                VerifiedTransactionInformation {
                    tx_prunable_blob,
                    tx_pruned: tx.serialize(),
                    tx_weight,
                    fee,
                    tx_hash,
                    tx,
                }
            })
            .collect(),
        pow_hash: prepped_block.pow_hash,
        generated_coins,
        weight: block_weight,
        height: context.chain_height,
        long_term_weight: context.next_block_long_term_weight(block_weight),
        cumulative_difficulty: context.cumulative_difficulty + context.next_difficulty,
    };

    if let Some(batch_prep_cache) = batch_prep_cache {
        batch_prep_cache.output_cache.add_block_to_cache(&block);
    }

    Ok(block)
}

//! The blockchain manager handler functions.
use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use monero_oxide::{block::Block, transaction::Input};
use rayon::prelude::*;
use tower::{Service, ServiceExt};
use tracing::{info, instrument, warn};

use cuprate_consensus::{
    block::{
        batch_prepare_main_chain_blocks, sanity_check_alt_block, verify_main_chain_block,
        verify_prepped_main_chain_block, BlockVerificationError, PreparedBlock,
    },
    transactions::new_tx_verification_data,
    BlockChainContextRequest, ExtendedConsensusError, VerificationContext,
};
use cuprate_consensus_context::{distribution::rct_output_count, BlockchainContext, NewBlockData};
use cuprate_fast_sync::{block_to_verified_block_information, fast_sync_stop_height};
use cuprate_helper::cast::usize_to_u64;
use cuprate_p2p::{block_downloader::BlockBatch, constants::MEDIUM_BAN, BroadcastRequest};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse, BlockchainWriteRequest},
    AltBlockInformation, Chain, ChainId, HardFork, TransactionVerificationData, TxConversionError,
    VerifiedBlockInformation,
};

use crate::{
    blockchain::{
        manager::commands::{BlockchainManagerCommand, IncomingBlockOk},
        ConsensusBlockchainReadHandle, IncomingBlockError,
    },
    monitor::FatalError,
};

impl super::BlockchainManager {
    /// Handle an incoming command from another part of Cuprate.
    ///
    /// # Errors
    ///
    /// This function will return an [`Err`] if any internal service returns an unexpected error.
    pub(super) async fn handle_command(
        &mut self,
        command: BlockchainManagerCommand,
    ) -> Result<(), FatalError> {
        match command {
            BlockchainManagerCommand::AddBlock {
                block,
                prepped_txs,
                response_tx,
            } => match self.handle_incoming_block(block, prepped_txs).await {
                Err(IncomingBlockError::Fatal(e)) => return Err(e),
                res => {
                    let _ = response_tx.send(res);
                }
            },
            BlockchainManagerCommand::PopBlocks {
                numb_blocks,
                response_tx,
            } => {
                let reorg_lock = Arc::clone(&self.reorg_lock);
                let _guard = reorg_lock.write().await;
                self.pop_blocks(numb_blocks, false).await?;
                self.blockchain_write_handle
                    .ready()
                    .await?
                    .call(BlockchainWriteRequest::FlushAltBlocks)
                    .await?;
                let _ = response_tx.send(());
            }
        }
        Ok(())
    }

    /// Broadcast a valid block to the network.
    async fn broadcast_block(&mut self, block_bytes: Bytes, blockchain_height: usize) {
        self.broadcast_svc
            .ready()
            .await
            .expect("Broadcast service is Infallible.")
            .call(BroadcastRequest::Block {
                block_bytes,
                current_blockchain_height: usize_to_u64(blockchain_height),
            })
            .await
            .expect("Broadcast service is Infallible.");
    }

    /// Handle an incoming [`Block`].
    ///
    /// This function will route to [`Self::handle_incoming_alt_block`] if the block does not follow
    /// the top of the main chain.
    ///
    /// Otherwise, this function will validate and add the block to the main chain.
    #[instrument(
        name = "incoming_block",
        skip_all,
        level = "info",
        fields(
            height = block.number(),
            txs = block.transactions.len(),
        )
    )]
    async fn handle_incoming_block(
        &mut self,
        block: Block,
        prepared_txs: HashMap<[u8; 32], TransactionVerificationData>,
    ) -> Result<IncomingBlockOk, IncomingBlockError> {
        if block.header.previous
            != self
                .blockchain_context_service
                .blockchain_context()
                .top_hash
        {
            let block_hash = block.hash();
            let res = self.handle_incoming_alt_block(block, prepared_txs).await?;

            if let AddAltBlock::NewlyCached(block_blob) = res {
                info!(
                    alt_block = true,
                    hash = hex::encode(block_hash),
                    "Successfully added block"
                );

                let chain_height = self
                    .blockchain_context_service
                    .blockchain_context()
                    .chain_height;

                self.broadcast_block(block_blob, chain_height).await;
            }

            return Ok(IncomingBlockOk::AddedToAltChain);
        }

        let verified_block = verify_main_chain_block(
            block,
            prepared_txs,
            &mut self.blockchain_context_service,
            self.blockchain_read_handle.clone(),
        )
        .await?;

        self.add_valid_block_to_main_chain(verified_block, BlockSource::Incoming)
            .await?;

        info!(
            hash = hex::encode(
                self.blockchain_context_service
                    .blockchain_context()
                    .top_hash
            ),
            "Successfully added block"
        );

        Ok(IncomingBlockOk::AddedToMainChain)
    }

    /// Handle an incoming [`BlockBatch`].
    ///
    /// This function will route to [`Self::handle_incoming_block_batch_main_chain`] or [`Self::handle_incoming_block_batch_alt_chain`]
    /// depending on if the first block in the batch follows from the top of our chain.
    ///
    /// # Errors
    ///
    /// This function will return an [`Err`] if any internal service returns an unexpected
    /// error that we cannot recover from.
    ///
    /// # Panics
    ///
    /// This function will panic if the incoming batch contains no blocks.
    #[instrument(
        name = "incoming_block_batch",
        skip_all,
        level = "info",
        fields(
            start_height = batch.blocks.first().unwrap().0.number(),
            len = batch.blocks.len()
        )
    )]
    pub(super) async fn handle_incoming_block_batch(
        &mut self,
        batch: BlockBatch,
    ) -> Result<(), FatalError> {
        let (first_block, _) = batch
            .blocks
            .first()
            .expect("Block batch should not be empty");

        if first_block.header.previous
            == self
                .blockchain_context_service
                .blockchain_context()
                .top_hash
        {
            self.handle_incoming_block_batch_main_chain(batch).await
        } else {
            self.handle_incoming_block_batch_alt_chain(batch).await
        }
    }

    /// Handles an incoming [`BlockBatch`] that follows the main chain.
    ///
    /// This function will handle validating the blocks in the batch and adding them to the blockchain
    /// database and context cache.
    ///
    /// This function will also handle banning the peer and canceling the block downloader if the
    /// block is invalid.
    ///
    /// # Errors
    ///
    /// This function will return an [`Err`] if any internal service returns an unexpected error that we cannot
    /// recover from.
    ///
    /// # Panics
    ///
    /// This function will panic if the incoming batch contains no blocks.
    async fn handle_incoming_block_batch_main_chain(
        &mut self,
        batch: BlockBatch,
    ) -> Result<(), FatalError> {
        let (last_block, _) = batch
            .blocks
            .last()
            .expect("Block batch should not be empty");

        if last_block.number() < fast_sync_stop_height(self.fast_sync_hashes) {
            return self.handle_incoming_block_batch_fast_sync(batch).await;
        }

        let (prepped_blocks, output_cache) = match batch_prepare_main_chain_blocks(
            batch.blocks,
            &mut self.blockchain_context_service,
            self.blockchain_read_handle.clone(),
        )
        .await
        {
            Ok(v) => v,
            Err(ExtendedConsensusError::FatalError(e)) => return Err(e),
            Err(ExtendedConsensusError::ConsensusError(e)) => {
                warn!("Failed to batch prepare blocks for verification: {e}, banning peer.");
                // This is not a gossiped block, we don't need to check if PoW was verified.
                batch.peer_handle.ban_peer(MEDIUM_BAN);
                self.stop_current_block_downloader.notify_waiters();
                return Ok(());
            }
        };

        let mut verification_context =
            VerificationContext::<ConsensusBlockchainReadHandle>::BatchPrepareCache(output_cache);
        let mut valid_blocks = Vec::with_capacity(prepped_blocks.len());
        let mut error = false;

        for (block, txs) in prepped_blocks {
            let hash = block.block_hash;
            let verified_block = match verify_prepped_main_chain_block(
                block,
                txs,
                &mut self.blockchain_context_service,
                &mut verification_context,
            )
            .await
            {
                Ok(block) => block,
                Err(e) => match e.into() {
                    ExtendedConsensusError::FatalError(e) => return Err(e),
                    ExtendedConsensusError::ConsensusError(e) => {
                        warn!(
                            "Failed to verify block: {}, error {}, banning peer.",
                            hex::encode(hash),
                            e
                        );

                        batch.peer_handle.ban_peer(MEDIUM_BAN);
                        self.stop_current_block_downloader.notify_waiters();
                        error = true;
                        break;
                    }
                },
            };

            self.add_valid_block_to_blockchain_cache(&verified_block)
                .await?;

            valid_blocks.push(verified_block);
        }

        let valid_blocks_len = valid_blocks.len();
        self.batch_add_valid_block_to_blockchain_database(valid_blocks)
            .await?;

        if error {
            info!(
                valid_blocks = valid_blocks_len,
                fast_sync = false,
                "Partially added block batch"
            );
        } else {
            info!(fast_sync = false, "Successfully added block batch");
        }
        Ok(())
    }

    /// Handles an incoming block batch while we are under the fast sync height.
    ///
    /// # Errors
    ///
    /// This function will return an [`Err`] if any internal service returns an unexpected error that we cannot
    /// recover from.
    async fn handle_incoming_block_batch_fast_sync(
        &mut self,
        batch: BlockBatch,
    ) -> Result<(), FatalError> {
        let mut valid_blocks = Vec::with_capacity(batch.blocks.len());
        for (block, txs) in batch.blocks {
            let block = block_to_verified_block_information(
                block,
                txs,
                self.blockchain_context_service.blockchain_context(),
            );
            self.add_valid_block_to_blockchain_cache(&block).await?;

            valid_blocks.push(block);
        }

        self.batch_add_valid_block_to_blockchain_database(valid_blocks)
            .await?;

        info!(fast_sync = true, "Successfully added block batch");
        Ok(())
    }

    /// Handles an incoming [`BlockBatch`] that does not follow the main-chain.
    ///
    /// This function will handle validating the alt-blocks to add them to our cache and reorging the
    /// chain if the alt-chain has a higher cumulative difficulty.
    ///
    /// This function will also handle banning the peer and canceling the block downloader if the
    /// alt block is invalid or if a reorg fails.
    ///
    /// # Errors
    ///
    /// This function will return an [`Err`] if any internal service returns an unexpected error that we cannot
    /// recover from.
    async fn handle_incoming_block_batch_alt_chain(
        &mut self,
        mut batch: BlockBatch,
    ) -> Result<(), FatalError> {
        let mut blocks = batch.blocks.into_iter();

        while let Some((block, txs)) = blocks.next() {
            let hash = block.hash();

            // async blocks work as try blocks.
            let res = async {
                let txs = txs
                    .into_par_iter()
                    .map(|tx| {
                        let tx = new_tx_verification_data(tx)?;
                        Ok((tx.tx_hash, tx))
                    })
                    .collect::<Result<_, ExtendedConsensusError>>()?;

                let reorged = self.handle_incoming_alt_block(block, txs).await?;

                Ok::<_, ExtendedConsensusError>(reorged)
            }
            .await;

            match res {
                Err(ExtendedConsensusError::FatalError(e)) => return Err(e),
                Err(ExtendedConsensusError::ConsensusError(e)) => {
                    warn!(
                        "Failed to verify alt block: {}, error {}, banning peer.",
                        hex::encode(hash),
                        e
                    );

                    batch.peer_handle.ban_peer(MEDIUM_BAN);
                    self.stop_current_block_downloader.notify_waiters();
                    return Ok(());
                }
                Ok(AddAltBlock::Reorged) => {
                    // Collect the remaining blocks and add them to the main chain instead.
                    batch.blocks = blocks.collect();

                    if batch.blocks.is_empty() {
                        return Ok(());
                    }

                    return self.handle_incoming_block_batch_main_chain(batch).await;
                }
                // continue adding alt blocks.
                Ok(
                    AddAltBlock::NewlyCached(_)
                    | AddAltBlock::AlreadyCached
                    | AddAltBlock::AlreadyInMainChain,
                ) => (),
            }
        }

        info!(alt_chain = true, "Successfully added block batch");
        Ok(())
    }

    /// Handles an incoming alt [`Block`].
    ///
    /// This function will do some pre-validation of the alt block, then if the cumulative difficulty
    /// of the alt chain is higher than the main chain it will attempt a reorg otherwise it will add
    /// the alt block to the alt block cache.
    ///
    /// # Errors
    ///
    /// This will return an [`Err`] if:
    ///  - The alt block was invalid.
    ///  - An attempt to reorg the chain failed.
    ///  - Any internal service returns an unexpected error.
    async fn handle_incoming_alt_block(
        &mut self,
        block: Block,
        prepared_txs: HashMap<[u8; 32], TransactionVerificationData>,
    ) -> Result<AddAltBlock, BlockVerificationError> {
        // Check if a block already exists.
        let BlockchainResponse::FindBlock(chain) = self
            .blockchain_read_handle
            .ready()
            .await
            .map_err(BlockVerificationError::fatal)?
            .call(BlockchainReadRequest::FindBlock(block.hash()))
            .await
            .map_err(BlockVerificationError::fatal)?
        else {
            unreachable!();
        };

        match chain {
            Some((Chain::Alt(_), _)) => return Ok(AddAltBlock::AlreadyCached),
            Some((Chain::Main, _)) => return Ok(AddAltBlock::AlreadyInMainChain),
            None => (),
        }

        let alt_block_info =
            sanity_check_alt_block(block, prepared_txs, self.blockchain_context_service.clone())
                .await?;

        // If this alt chain has more cumulative difficulty, reorg.
        if alt_block_info.cumulative_difficulty
            > self
                .blockchain_context_service
                .blockchain_context()
                .cumulative_difficulty
        {
            self.try_do_reorg(alt_block_info)
                .await
                .map_err(BlockVerificationError::valid_pow)?;
            return Ok(AddAltBlock::Reorged);
        }

        let block_blob = Bytes::copy_from_slice(&alt_block_info.block_blob);
        self.blockchain_write_handle
            .ready()
            .await
            .map_err(|e| BlockVerificationError::fatal(e.into()))?
            .call(BlockchainWriteRequest::WriteAltBlock(alt_block_info))
            .await
            .map_err(|e| BlockVerificationError::fatal(e.into()))?;

        Ok(AddAltBlock::NewlyCached(block_blob))
    }

    /// Attempt a re-org with the given top block of the alt-chain.
    ///
    /// This function will take a write lock on `reorg_lock` and then set up the blockchain database
    /// and context cache to verify the alt-chain. It will then attempt to verify and add each block
    /// in the alt-chain to the main-chain. Releasing the lock on `reorg_lock` when finished.
    ///
    /// # Errors
    ///
    /// This function will return an [`Err`] if any internal service returns an unexpected error,
    /// or if the re-org was unsuccessful. If this happens the chain
    /// will be returned to the state it was in when the function was called.
    #[instrument(name = "try_do_reorg", skip_all, level = "info")]
    async fn try_do_reorg(
        &mut self,
        top_alt_block: AltBlockInformation,
    ) -> Result<(), ExtendedConsensusError> {
        let reorg_lock = Arc::clone(&self.reorg_lock);
        let _guard = reorg_lock.write().await;

        let BlockchainResponse::AltBlocksInChain(mut alt_blocks) = self
            .blockchain_read_handle
            .ready()
            .await?
            .call(BlockchainReadRequest::AltBlocksInChain(
                top_alt_block.chain_id,
            ))
            .await?
        else {
            unreachable!();
        };

        alt_blocks.push(top_alt_block);

        let split_height = alt_blocks[0].height;
        let current_main_chain_height = self
            .blockchain_context_service
            .blockchain_context()
            .chain_height;

        info!(split_height, "Attempting blockchain reorg");

        let old_main_chain_id = self
            .pop_blocks(current_main_chain_height - split_height, true)
            .await?;

        let reorg_res = self.verify_add_alt_blocks_to_main_chain(alt_blocks).await;

        match reorg_res {
            Ok(()) => {
                info!(
                    top_hash = hex::encode(
                        self.blockchain_context_service
                            .blockchain_context()
                            .top_hash
                    ),
                    "Successfully reorged"
                );
                Ok(())
            }
            Err(e) => {
                if let Some(old_main_chain_id) = old_main_chain_id {
                    self.reverse_reorg(old_main_chain_id).await?;
                } else {
                    warn!("Failed to revert reorg, reorg removed pruned blocks which we cannot add back.");
                }
                Err(e)
            }
        }
    }

    /// Reverse a reorg that failed.
    ///
    /// This function takes the old chain's [`ChainId`] and reverts the chain state to back to before
    /// the reorg was attempted.
    ///
    /// # Errors
    ///
    /// This function will return an [`Err`] if any internal service returns an unexpected error that we cannot
    /// recover from.
    #[instrument(name = "reverse_reorg", skip_all, level = "info")]
    async fn reverse_reorg(&mut self, old_main_chain_id: ChainId) -> Result<(), FatalError> {
        warn!("Reorg failed, reverting to old chain.");

        let BlockchainResponse::AltBlocksInChain(blocks) = self
            .blockchain_read_handle
            .ready()
            .await?
            .call(BlockchainReadRequest::AltBlocksInChain(old_main_chain_id))
            .await?
        else {
            unreachable!();
        };

        let split_height = blocks[0].height;
        let current_main_chain_height = self
            .blockchain_context_service
            .blockchain_context()
            .chain_height;

        let numb_blocks = current_main_chain_height - split_height;

        if numb_blocks > 0 {
            self.pop_blocks(current_main_chain_height - split_height, false)
                .await?;
        }

        for block in blocks {
            let verified_block = alt_block_to_verified_block_information(
                block,
                self.blockchain_context_service.blockchain_context(),
            );
            self.add_valid_block_to_main_chain(verified_block, BlockSource::Reorg)
                .await?;
        }

        self.blockchain_write_handle
            .ready()
            .await?
            .call(BlockchainWriteRequest::FlushAltBlocks)
            .await?;

        info!("Successfully reversed reorg");
        Ok(())
    }

    /// Pop blocks from the main chain, moving them to alt-blocks. This function will flush all other alt-blocks.
    ///
    /// This returns the [`ChainId`] of the blocks that were popped.
    ///
    /// # Errors
    ///
    /// This function will return an [`Err`] if any internal service returns an unexpected error that we cannot
    /// recover from.
    #[instrument(name = "pop_blocks", skip(self), level = "info")]
    async fn pop_blocks(
        &mut self,
        numb_blocks: usize,
        keep: bool,
    ) -> Result<Option<ChainId>, FatalError> {
        let BlockchainResponse::PopBlocks(old_main_chain_id) = self
            .blockchain_write_handle
            .ready()
            .await?
            .call(BlockchainWriteRequest::PopBlocks(numb_blocks, keep))
            .await?
        else {
            unreachable!();
        };

        self.blockchain_context_service
            .ready()
            .await?
            .call(BlockChainContextRequest::PopBlocks { numb_blocks })
            .await?;

        Ok(old_main_chain_id)
    }

    /// Verify and add a list of [`AltBlockInformation`]s to the main-chain.
    ///
    /// This function assumes the first [`AltBlockInformation`] is the next block in the blockchain
    /// for the blockchain database and the context cache, or in other words that the blockchain database
    /// and context cache have already had the top blocks popped to where the alt-chain meets the main-chain.
    ///
    /// # Errors
    ///
    /// This function will return an [`Err`] if any internal service returns an unexpected error,
    /// or if the alt-blocks are invalid. In this case the re-org should be aborted and the chain
    /// returned to its previous state.
    async fn verify_add_alt_blocks_to_main_chain(
        &mut self,
        alt_blocks: Vec<AltBlockInformation>,
    ) -> Result<(), ExtendedConsensusError> {
        for mut alt_block in alt_blocks {
            let prepped_txs = alt_block
                .txs
                .drain(..)
                .map(TryInto::try_into)
                .collect::<Result<_, TxConversionError>>()
                .map_err(|e| ExtendedConsensusError::FatalError(e.into()))?;

            let prepped_block = PreparedBlock::new_alt_block(alt_block)?;

            let verified_block = verify_prepped_main_chain_block(
                prepped_block,
                prepped_txs,
                &mut self.blockchain_context_service,
                &mut VerificationContext::Database(self.blockchain_read_handle.clone()),
            )
            .await?;

            self.add_valid_block_to_main_chain(verified_block, BlockSource::Reorg)
                .await?;
        }

        Ok(())
    }

    /// Adds a [`VerifiedBlockInformation`] to the main-chain.
    ///
    /// This function will update the blockchain database and the context cache,
    /// and announce the block to peers if `source` is [`BlockSource::Incoming`].
    ///
    /// # Errors
    ///
    /// This function will return an [`Err`] if any internal service returns an unexpected error that we cannot
    /// recover from.
    async fn add_valid_block_to_main_chain(
        &mut self,
        verified_block: VerifiedBlockInformation,
        source: BlockSource,
    ) -> Result<(), FatalError> {
        // FIXME: this is pretty inefficient, we should probably return the KI map created in the consensus crate.
        let spent_key_images = verified_block
            .txs
            .iter()
            .flat_map(|tx| {
                tx.tx.prefix().inputs.iter().map(|input| match input {
                    Input::ToKey { key_image, .. } => key_image.to_bytes(),
                    Input::Gen(_) => unreachable!(),
                })
            })
            .collect::<Vec<[u8; 32]>>();

        let block_blob = matches!(source, BlockSource::Incoming)
            .then(|| Bytes::copy_from_slice(&verified_block.block_blob));

        self.add_valid_block_to_blockchain_cache(&verified_block)
            .await?;

        self.add_valid_block_to_blockchain_database(verified_block)
            .await?;

        if let Some(block_blob) = block_blob {
            let chain_height = self
                .blockchain_context_service
                .blockchain_context()
                .chain_height;

            self.broadcast_block(block_blob, chain_height).await;
        }

        self.txpool_manager_handle
            .new_block(spent_key_images)
            .await?;

        Ok(())
    }

    /// Adds a [`VerifiedBlockInformation`] to the blockchain context cache.
    ///
    /// # Errors
    ///
    /// This function will return an [`Err`] if any internal service returns an unexpected error that we cannot
    /// recover from.
    async fn add_valid_block_to_blockchain_cache(
        &mut self,
        verified_block: &VerifiedBlockInformation,
    ) -> Result<(), FatalError> {
        self.blockchain_context_service
            .ready()
            .await?
            .call(BlockChainContextRequest::Update(NewBlockData {
                block_hash: verified_block.block_hash,
                height: verified_block.height,
                timestamp: verified_block.block.header.timestamp,
                weight: verified_block.weight,
                long_term_weight: verified_block.long_term_weight,
                generated_coins: verified_block.generated_coins,
                vote: HardFork::from_vote(verified_block.block.header.hardfork_signal),
                cumulative_difficulty: verified_block.cumulative_difficulty,
                numb_rct_outputs: rct_output_count(verified_block),
            }))
            .await?;
        Ok(())
    }

    /// Writes a [`VerifiedBlockInformation`] to the blockchain database.
    ///
    /// # Errors
    ///
    /// This function will return an [`Err`] if any internal service returns an unexpected error that we cannot
    /// recover from.
    async fn add_valid_block_to_blockchain_database(
        &mut self,
        verified_block: VerifiedBlockInformation,
    ) -> Result<(), FatalError> {
        self.blockchain_write_handle
            .ready()
            .await?
            .call(BlockchainWriteRequest::WriteBlock(verified_block))
            .await?;
        Ok(())
    }

    /// Batch writes the [`VerifiedBlockInformation`]s to the database.
    ///
    /// The blocks must be sequential.
    ///
    /// # Errors
    ///
    /// This function will return an [`Err`] if any internal service returns an unexpected error that we cannot
    /// recover from.
    async fn batch_add_valid_block_to_blockchain_database(
        &mut self,
        blocks: Vec<VerifiedBlockInformation>,
    ) -> Result<(), FatalError> {
        self.blockchain_write_handle
            .ready()
            .await?
            .call(BlockchainWriteRequest::BatchWriteBlocks(blocks))
            .await?;
        Ok(())
    }
}

/// The result from successfully adding an alt-block.
enum AddAltBlock {
    /// We already had this alt-block cached.
    AlreadyCached,
    /// The block already exists on the main chain.
    AlreadyInMainChain,
    /// The alt-block was newly cached. Contains the block blob.
    NewlyCached(Bytes),
    /// The chain was reorged.
    Reorged,
}

/// The context in which a verified block is being added to the main chain.
enum BlockSource {
    /// A single incoming block. Will be announced to peers.
    Incoming,
    /// A block re-applied during a reorg.
    Reorg,
}

/// Creates a [`VerifiedBlockInformation`] from an alt-block known to be valid.
///
/// # Panics
///
/// This may panic if used on an invalid block.
fn alt_block_to_verified_block_information(
    block: AltBlockInformation,
    blockchain_ctx: &BlockchainContext,
) -> VerifiedBlockInformation {
    assert_eq!(
        block.height, blockchain_ctx.chain_height,
        "alt-block invalid"
    );

    let total_fees = block.txs.iter().map(|tx| tx.fee).sum::<u64>();
    let total_outputs = block
        .block
        .miner_transaction()
        .prefix()
        .outputs
        .iter()
        .map(|output| output.amount.unwrap_or(0))
        .sum::<u64>();

    let generated_coins = total_outputs - total_fees;

    VerifiedBlockInformation {
        block_blob: block.block_blob,
        txs: block.txs,
        block_hash: block.block_hash,
        pow_hash: [u8::MAX; 32],
        height: block.height,
        generated_coins,
        weight: block.weight,
        long_term_weight: blockchain_ctx.next_block_long_term_weight(block.weight),
        cumulative_difficulty: blockchain_ctx.cumulative_difficulty
            + blockchain_ctx.next_difficulty,
        block: block.block,
    }
}

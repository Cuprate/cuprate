//! The blockchain manager handler functions.
use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use futures::{TryFutureExt, TryStreamExt};
use monero_oxide::{
    block::Block,
    transaction::{Input, Transaction},
};
use rayon::prelude::*;
use tower::{Service, ServiceExt};
use tracing::{info, instrument, warn, Span};

use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::{
    block::{
        batch_prepare_main_chain_blocks, sanity_check_alt_block, verify_main_chain_block,
        verify_prepped_main_chain_block, PreparedBlock,
    },
    transactions::new_tx_verification_data,
    BlockChainContextRequest, BlockChainContextResponse, ExtendedConsensusError,
};
use cuprate_consensus_context::{BlockchainContext, NewBlockData};
use cuprate_fast_sync::{block_to_verified_block_information, fast_sync_stop_height};
use cuprate_helper::cast::usize_to_u64;
use cuprate_p2p::{block_downloader::BlockBatch, constants::LONG_BAN, BroadcastRequest};
use cuprate_txpool::service::interface::TxpoolWriteRequest;
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse, BlockchainWriteRequest},
    AltBlockInformation, Chain, ChainId, HardFork, TransactionVerificationData,
    VerifiedBlockInformation, VerifiedTransactionInformation,
};

use crate::{
    blockchain::manager::commands::{BlockchainManagerCommand, IncomingBlockOk},
    constants::PANIC_CRITICAL_SERVICE_ERROR,
    signals::REORG_LOCK,
};

impl super::BlockchainManager {
    /// Handle an incoming command from another part of Cuprate.
    ///
    /// # Panics
    ///
    /// This function will panic if any internal service returns an unexpected error that we cannot
    /// recover from.
    pub async fn handle_command(&mut self, command: BlockchainManagerCommand) {
        match command {
            BlockchainManagerCommand::AddBlock {
                block,
                prepped_txs,
                response_tx,
            } => {
                let res = self.handle_incoming_block(block, prepped_txs).await;

                drop(response_tx.send(res));
            }
        }
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
    ///
    /// # Panics
    ///
    /// This function will panic if any internal service returns an unexpected error that we cannot
    /// recover from.
    #[instrument(
        name = "incoming_block",
        skip_all,
        level = "info",
        fields(
            height = block.number().unwrap(),
            txs = block.transactions.len(),
        )
    )]
    pub async fn handle_incoming_block(
        &mut self,
        block: Block,
        prepared_txs: HashMap<[u8; 32], TransactionVerificationData>,
    ) -> Result<IncomingBlockOk, anyhow::Error> {
        if block.header.previous
            != self
                .blockchain_context_service
                .blockchain_context()
                .top_hash
        {
            let block_hash = block.hash();
            let res = self.handle_incoming_alt_block(block, prepared_txs).await?;

            if matches!(res, AddAltBlock::Cached(true)) {
                info!(
                    alt_block = true,
                    hash = hex::encode(block_hash),
                    "Successfully added block"
                );
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

        let block_blob = Bytes::copy_from_slice(&verified_block.block_blob);
        self.add_valid_block_to_main_chain(verified_block).await;

        let chain_height = self
            .blockchain_context_service
            .blockchain_context()
            .chain_height;

        self.broadcast_block(block_blob, chain_height).await;

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
    /// # Panics
    ///
    /// This function will panic if the batch is empty or if any internal service returns an unexpected
    /// error that we cannot recover from or if the incoming batch contains no blocks.
    #[instrument(
        name = "incoming_block_batch",
        skip_all,
        level = "info",
        fields(
            start_height = batch.blocks.first().unwrap().0.number().unwrap(),
            len = batch.blocks.len()
        )
    )]
    pub async fn handle_incoming_block_batch(&mut self, batch: BlockBatch) {
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
            self.handle_incoming_block_batch_main_chain(batch).await;
        } else {
            self.handle_incoming_block_batch_alt_chain(batch).await;
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
    /// # Panics
    ///
    /// This function will panic if any internal service returns an unexpected error that we cannot
    /// recover from or if the incoming batch contains no blocks.
    async fn handle_incoming_block_batch_main_chain(&mut self, batch: BlockBatch) {
        if batch.blocks.last().unwrap().0.number().unwrap() < fast_sync_stop_height() {
            self.handle_incoming_block_batch_fast_sync(batch).await;
            return;
        }

        let Ok((prepped_blocks, mut output_cache)) = batch_prepare_main_chain_blocks(
            batch.blocks,
            &mut self.blockchain_context_service,
            self.blockchain_read_handle.clone(),
        )
        .await
        else {
            batch.peer_handle.ban_peer(LONG_BAN);
            self.stop_current_block_downloader.notify_one();
            return;
        };

        for (block, txs) in prepped_blocks {
            let Ok(verified_block) = verify_prepped_main_chain_block(
                block,
                txs,
                &mut self.blockchain_context_service,
                self.blockchain_read_handle.clone(),
                Some(&mut output_cache),
            )
            .await
            else {
                batch.peer_handle.ban_peer(LONG_BAN);
                self.stop_current_block_downloader.notify_one();
                return;
            };

            self.add_valid_block_to_main_chain(verified_block).await;
        }
        info!(fast_sync = false, "Successfully added block batch");
    }

    /// Handles an incoming block batch while we are under the fast sync height.
    ///
    /// # Panics
    ///
    /// This function will panic if any internal service returns an unexpected error that we cannot
    /// recover from.
    async fn handle_incoming_block_batch_fast_sync(&mut self, batch: BlockBatch) {
        let mut valid_blocks = Vec::with_capacity(batch.blocks.len());
        for (block, txs) in batch.blocks {
            let block = block_to_verified_block_information(
                block,
                txs,
                self.blockchain_context_service.blockchain_context(),
            );
            self.add_valid_block_to_blockchain_cache(&block).await;

            valid_blocks.push(block);
        }

        self.batch_add_valid_block_to_blockchain_database(valid_blocks)
            .await;

        info!(fast_sync = true, "Successfully added block batch");
    }

    /// Handles an incoming [`BlockBatch`] that does not follow the main-chain.
    ///
    /// This function will handle validating the alt-blocks to add them to our cache and reorging the
    /// chain if the alt-chain has a higher cumulative difficulty.
    ///
    /// This function will also handle banning the peer and canceling the block downloader if the
    /// alt block is invalid or if a reorg fails.
    ///
    /// # Panics
    ///
    /// This function will panic if any internal service returns an unexpected error that we cannot
    /// recover from.
    async fn handle_incoming_block_batch_alt_chain(&mut self, mut batch: BlockBatch) {
        let mut blocks = batch.blocks.into_iter();

        while let Some((block, txs)) = blocks.next() {
            // async blocks work as try blocks.
            let res = async {
                let txs = txs
                    .into_par_iter()
                    .map(|tx| {
                        let tx = new_tx_verification_data(tx)?;
                        Ok((tx.tx_hash, tx))
                    })
                    .collect::<Result<_, anyhow::Error>>()?;

                let reorged = self.handle_incoming_alt_block(block, txs).await?;

                Ok::<_, anyhow::Error>(reorged)
            }
            .await;

            match res {
                Err(e) => {
                    batch.peer_handle.ban_peer(LONG_BAN);
                    self.stop_current_block_downloader.notify_one();
                    return;
                }
                Ok(AddAltBlock::Reorged) => {
                    // Collect the remaining blocks and add them to the main chain instead.
                    batch.blocks = blocks.collect();

                    if batch.blocks.is_empty() {
                        return;
                    }

                    self.handle_incoming_block_batch_main_chain(batch).await;
                    return;
                }
                // continue adding alt blocks.
                Ok(AddAltBlock::Cached(_)) => (),
            }
        }

        info!(alt_chain = true, "Successfully added block batch");
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
    ///
    /// # Panics
    ///
    /// This function will panic if any internal service returns an unexpected error that we cannot
    /// recover from.
    async fn handle_incoming_alt_block(
        &mut self,
        block: Block,
        prepared_txs: HashMap<[u8; 32], TransactionVerificationData>,
    ) -> Result<AddAltBlock, anyhow::Error> {
        // Check if a block already exists.
        let BlockchainResponse::FindBlock(chain) = self
            .blockchain_read_handle
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(BlockchainReadRequest::FindBlock(block.hash()))
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
        else {
            unreachable!();
        };

        match chain {
            Some((Chain::Alt(_), _)) => return Ok(AddAltBlock::Cached(false)),
            Some((Chain::Main, _)) => anyhow::bail!("Alt block already in main chain"),
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
            self.try_do_reorg(alt_block_info).await?;
            return Ok(AddAltBlock::Reorged);
        }

        self.blockchain_write_handle
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(BlockchainWriteRequest::WriteAltBlock(alt_block_info))
            .await?;

        Ok(AddAltBlock::Cached(true))
    }

    /// Attempt a re-org with the given top block of the alt-chain.
    ///
    /// This function will take a write lock on [`REORG_LOCK`] and then set up the blockchain database
    /// and context cache to verify the alt-chain. It will then attempt to verify and add each block
    /// in the alt-chain to the main-chain. Releasing the lock on [`REORG_LOCK`] when finished.
    ///
    /// # Errors
    ///
    /// This function will return an [`Err`] if the re-org was unsuccessful, if this happens the chain
    /// will be returned back into its state it was at when then function was called.
    ///
    /// # Panics
    ///
    /// This function will panic if any internal service returns an unexpected error that we cannot
    /// recover from.
    #[instrument(name = "try_do_reorg", skip_all, level = "info")]
    async fn try_do_reorg(
        &mut self,
        top_alt_block: AltBlockInformation,
    ) -> Result<(), anyhow::Error> {
        let _guard = REORG_LOCK.write().await;

        let BlockchainResponse::AltBlocksInChain(mut alt_blocks) = self
            .blockchain_read_handle
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(BlockchainReadRequest::AltBlocksInChain(
                top_alt_block.chain_id,
            ))
            .await
            .map_err(|e| anyhow::anyhow!(e))?
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
            .pop_blocks(current_main_chain_height - split_height)
            .await;

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
                self.reverse_reorg(old_main_chain_id).await;
                Err(e)
            }
        }
    }

    /// Reverse a reorg that failed.
    ///
    /// This function takes the old chain's [`ChainId`] and reverts the chain state to back to before
    /// the reorg was attempted.
    ///
    /// # Panics
    ///
    /// This function will panic if any internal service returns an unexpected error that we cannot
    /// recover from.
    #[instrument(name = "reverse_reorg", skip_all, level = "info")]
    async fn reverse_reorg(&mut self, old_main_chain_id: ChainId) {
        warn!("Reorg failed, reverting to old chain.");

        let BlockchainResponse::AltBlocksInChain(mut blocks) = self
            .blockchain_read_handle
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(BlockchainReadRequest::AltBlocksInChain(old_main_chain_id))
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
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
            self.pop_blocks(current_main_chain_height - split_height)
                .await;
        }

        for block in blocks {
            let verified_block = alt_block_to_verified_block_information(
                block,
                self.blockchain_context_service.blockchain_context(),
            );
            self.add_valid_block_to_main_chain(verified_block).await;
        }

        self.blockchain_write_handle
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(BlockchainWriteRequest::FlushAltBlocks)
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR);

        info!("Successfully reversed reorg");
    }

    /// Pop blocks from the main chain, moving them to alt-blocks. This function will flush all other alt-blocks.
    ///
    /// This returns the [`ChainId`] of the blocks that were popped.
    ///
    /// # Panics
    ///
    /// This function will panic if any internal service returns an unexpected error that we cannot
    /// recover from.
    #[instrument(name = "pop_blocks", skip(self), level = "info")]
    async fn pop_blocks(&mut self, numb_blocks: usize) -> ChainId {
        let BlockchainResponse::PopBlocks(old_main_chain_id) = self
            .blockchain_write_handle
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(BlockchainWriteRequest::PopBlocks(numb_blocks))
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
        else {
            unreachable!();
        };

        self.blockchain_context_service
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(BlockChainContextRequest::PopBlocks { numb_blocks })
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR);

        old_main_chain_id
    }

    /// Verify and add a list of [`AltBlockInformation`]s to the main-chain.
    ///
    /// This function assumes the first [`AltBlockInformation`] is the next block in the blockchain
    /// for the blockchain database and the context cache, or in other words that the blockchain database
    /// and context cache have already had the top blocks popped to where the alt-chain meets the main-chain.
    ///
    /// # Errors
    ///
    /// This function will return an [`Err`] if the alt-blocks were invalid, in this case the re-org should
    /// be aborted and the chain should be returned to its previous state.
    ///
    /// # Panics
    ///
    /// This function will panic if any internal service returns an unexpected error that we cannot
    /// recover from.
    async fn verify_add_alt_blocks_to_main_chain(
        &mut self,
        alt_blocks: Vec<AltBlockInformation>,
    ) -> Result<(), anyhow::Error> {
        for mut alt_block in alt_blocks {
            let prepped_txs = alt_block
                .txs
                .drain(..)
                .map(|tx| Ok(tx.try_into()?))
                .collect::<Result<_, anyhow::Error>>()?;

            let prepped_block = PreparedBlock::new_alt_block(alt_block)?;

            let verified_block = verify_prepped_main_chain_block(
                prepped_block,
                prepped_txs,
                &mut self.blockchain_context_service,
                self.blockchain_read_handle.clone(),
                None,
            )
            .await?;

            self.add_valid_block_to_main_chain(verified_block).await;
        }

        Ok(())
    }

    /// Adds a [`VerifiedBlockInformation`] to the main-chain.
    ///
    /// This function will update the blockchain database and the context cache.
    ///
    /// # Panics
    ///
    /// This function will panic if any internal service returns an unexpected error that we cannot
    /// recover from.
    pub async fn add_valid_block_to_main_chain(
        &mut self,
        verified_block: VerifiedBlockInformation,
    ) {
        // FIXME: this is pretty inefficient, we should probably return the KI map created in the consensus crate.
        let spent_key_images = verified_block
            .txs
            .iter()
            .flat_map(|tx| {
                tx.tx.prefix().inputs.iter().map(|input| match input {
                    Input::ToKey { key_image, .. } => key_image.0,
                    Input::Gen(_) => unreachable!(),
                })
            })
            .collect::<Vec<[u8; 32]>>();

        self.add_valid_block_to_blockchain_cache(&verified_block)
            .await;

        self.blockchain_write_handle
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(BlockchainWriteRequest::WriteBlock(verified_block))
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR);

        self.txpool_manager_handle
            .new_block(spent_key_images)
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR);
    }

    /// Adds a [`VerifiedBlockInformation`] to the blockchain context cache.
    ///
    /// # Panics
    ///
    /// This function will panic if any internal service returns an unexpected error that we cannot
    /// recover from.
    async fn add_valid_block_to_blockchain_cache(
        &mut self,
        verified_block: &VerifiedBlockInformation,
    ) {
        self.blockchain_context_service
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(BlockChainContextRequest::Update(NewBlockData {
                block_hash: verified_block.block_hash,
                height: verified_block.height,
                timestamp: verified_block.block.header.timestamp,
                weight: verified_block.weight,
                long_term_weight: verified_block.long_term_weight,
                generated_coins: verified_block.generated_coins,
                vote: HardFork::from_vote(verified_block.block.header.hardfork_signal),
                cumulative_difficulty: verified_block.cumulative_difficulty,
            }))
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR);
    }

    /// Batch writes the [`VerifiedBlockInformation`]s to the database.
    ///
    /// The blocks must be sequential.
    ///
    /// # Panics
    ///
    /// This function will panic if any internal service returns an unexpected error that we cannot
    /// recover from.
    async fn batch_add_valid_block_to_blockchain_database(
        &mut self,
        blocks: Vec<VerifiedBlockInformation>,
    ) {
        self.blockchain_write_handle
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(BlockchainWriteRequest::BatchWriteBlocks(blocks))
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR);
    }
}

/// The result from successfully adding an alt-block.
enum AddAltBlock {
    /// The alt-block was cached.
    ///
    /// The inner `bool` is for if the block was cached before [`false`] or was cached during the call [`true`].
    Cached(bool),
    /// The chain was reorged.
    Reorged,
}

/// Creates a [`VerifiedBlockInformation`] from an alt-block known to be valid.
///
/// # Panics
///
/// This may panic if used on an invalid block.
pub fn alt_block_to_verified_block_information(
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
        .miner_transaction
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

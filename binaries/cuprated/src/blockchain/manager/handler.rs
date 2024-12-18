//! The blockchain manager handler functions.
use bytes::Bytes;
use futures::{TryFutureExt, TryStreamExt};
use monero_serai::{
    block::Block,
    transaction::{Input, Transaction},
};
use rayon::prelude::*;
use std::ops::ControlFlow;
use std::{collections::HashMap, sync::Arc};
use tower::{Service, ServiceExt};
use tracing::{info, instrument};

use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::{
    block::PreparedBlock, transactions::new_tx_verification_data, BlockChainContextRequest,
    BlockChainContextResponse, BlockVerifierService, ExtendedConsensusError, VerifyBlockRequest,
    VerifyBlockResponse, VerifyTxRequest, VerifyTxResponse,
};
use cuprate_consensus_context::NewBlockData;
use cuprate_helper::cast::usize_to_u64;
use cuprate_p2p::{block_downloader::BlockBatch, constants::LONG_BAN, BroadcastRequest};
use cuprate_txpool::service::interface::TxpoolWriteRequest;
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse, BlockchainWriteRequest},
    AltBlockInformation, HardFork, TransactionVerificationData, VerifiedBlockInformation,
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
    pub async fn handle_incoming_block(
        &mut self,
        block: Block,
        prepared_txs: HashMap<[u8; 32], TransactionVerificationData>,
    ) -> Result<IncomingBlockOk, anyhow::Error> {
        if block.header.previous != self.cached_blockchain_context.top_hash {
            self.handle_incoming_alt_block(block, prepared_txs).await?;
            return Ok(IncomingBlockOk::AddedToAltChain);
        }

        let VerifyBlockResponse::MainChain(verified_block) = self
            .block_verifier_service
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(VerifyBlockRequest::MainChain {
                block,
                prepared_txs,
            })
            .await?
        else {
            unreachable!();
        };

        let block_blob = Bytes::copy_from_slice(&verified_block.block_blob);
        self.add_valid_block_to_main_chain(verified_block).await;

        self.broadcast_block(block_blob, self.cached_blockchain_context.chain_height)
            .await;

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
    #[instrument(name = "incoming_block_batch" skip_all, level = "info", fields(start_height = batch.blocks.first().unwrap().0.number().unwrap(), len = batch.blocks.len()))]
    pub async fn handle_incoming_block_batch(&mut self, batch: BlockBatch) {
        let (first_block, _) = batch
            .blocks
            .first()
            .expect("Block batch should not be empty");

        if first_block.header.previous == self.cached_blockchain_context.top_hash {
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
        let start_height = batch.blocks.first().unwrap().0.number().unwrap();

        let batch_prep_res = self
            .block_verifier_service
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(VerifyBlockRequest::MainChainBatchPrepareBlocks {
                blocks: batch.blocks,
            })
            .await;

        let prepped_blocks = match batch_prep_res {
            Ok(VerifyBlockResponse::MainChainBatchPrepped(prepped_blocks)) => prepped_blocks,
            Err(_) => {
                batch.peer_handle.ban_peer(LONG_BAN);
                self.stop_current_block_downloader.notify_one();
                return;
            }
            _ => unreachable!(),
        };

        for (block, txs) in prepped_blocks {
            let verify_res = self
                .block_verifier_service
                .ready()
                .await
                .expect(PANIC_CRITICAL_SERVICE_ERROR)
                .call(VerifyBlockRequest::MainChainPrepped { block, txs })
                .await;

            let verified_block = match verify_res {
                Ok(VerifyBlockResponse::MainChain(verified_block)) => verified_block,
                Err(_) => {
                    batch.peer_handle.ban_peer(LONG_BAN);
                    self.stop_current_block_downloader.notify_one();
                    return;
                }
                _ => unreachable!(),
            };

            self.add_valid_block_to_main_chain(verified_block).await;
        }
        info!("Successfully added block batch");
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
        // TODO: this needs testing (this whole section does but alt-blocks specifically).

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
                    self.handle_incoming_block_batch_main_chain(batch).await;
                    return;
                }
                // continue adding alt blocks.
                Ok(AddAltBlock::Cached) => (),
            }
        }
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
        let VerifyBlockResponse::AltChain(alt_block_info) = self
            .block_verifier_service
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(VerifyBlockRequest::AltChain {
                block,
                prepared_txs,
            })
            .await?
        else {
            unreachable!();
        };

        // TODO: check in consensus crate if alt block with this hash already exists.

        // If this alt chain
        if alt_block_info.cumulative_difficulty
            > self.cached_blockchain_context.cumulative_difficulty
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

        Ok(AddAltBlock::Cached)
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
            .await?
        else {
            unreachable!();
        };

        alt_blocks.push(top_alt_block);

        let split_height = alt_blocks[0].height;
        let current_main_chain_height = self.cached_blockchain_context.chain_height;

        let BlockchainResponse::PopBlocks(old_main_chain_id) = self
            .blockchain_write_handle
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(BlockchainWriteRequest::PopBlocks(
                current_main_chain_height - split_height + 1,
            ))
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
        else {
            unreachable!();
        };

        self.blockchain_context_service
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(BlockChainContextRequest::PopBlocks {
                numb_blocks: current_main_chain_height - split_height + 1,
            })
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR);

        let reorg_res = self.verify_add_alt_blocks_to_main_chain(alt_blocks).await;

        match reorg_res {
            Ok(()) => Ok(()),
            Err(e) => {
                todo!("Reverse reorg")
            }
        }
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
                .map(|tx| Ok(Arc::new(tx.try_into()?)))
                .collect::<Result<_, anyhow::Error>>()?;

            let prepped_block = PreparedBlock::new_alt_block(alt_block)?;

            let VerifyBlockResponse::MainChain(verified_block) = self
                .block_verifier_service
                .ready()
                .await
                .expect(PANIC_CRITICAL_SERVICE_ERROR)
                .call(VerifyBlockRequest::MainChainPrepped {
                    block: prepped_block,
                    txs: prepped_txs,
                })
                .await?
            else {
                unreachable!();
            };

            self.add_valid_block_to_main_chain(verified_block).await;
        }

        Ok(())
    }

    /// Adds a [`VerifiedBlockInformation`] to the main-chain.
    ///
    /// This function will update the blockchain database and the context cache, it will also
    /// update [`Self::cached_blockchain_context`].
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
                    Input::ToKey { key_image, .. } => key_image.compress().0,
                    Input::Gen(_) => unreachable!(),
                })
            })
            .collect::<Vec<[u8; 32]>>();

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

        self.blockchain_write_handle
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(BlockchainWriteRequest::WriteBlock(verified_block))
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR);

        let BlockChainContextResponse::Context(blockchain_context) = self
            .blockchain_context_service
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(BlockChainContextRequest::Context)
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
        else {
            unreachable!();
        };

        self.cached_blockchain_context = blockchain_context.unchecked_blockchain_context().clone();

        self.txpool_write_handle
            .ready()
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR)
            .call(TxpoolWriteRequest::NewBlock { spent_key_images })
            .await
            .expect(PANIC_CRITICAL_SERVICE_ERROR);
    }
}

/// The result from successfully adding an alt-block.
enum AddAltBlock {
    /// The alt-block was cached.
    Cached,
    /// The chain was reorged.
    Reorged,
}

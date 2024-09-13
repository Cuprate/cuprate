use std::{collections::HashMap, sync::Arc};

use futures::{TryFutureExt, TryStreamExt};
use monero_serai::{block::Block, transaction::Transaction};
use rayon::prelude::*;
use tower::{Service, ServiceExt};
use tracing::info;

use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::{
    block::PreparedBlock, context::NewBlockData, transactions::new_tx_verification_data,
    BlockChainContextRequest, BlockChainContextResponse, BlockVerifierService,
    ExtendedConsensusError, VerifyBlockRequest, VerifyBlockResponse, VerifyTxRequest,
    VerifyTxResponse,
};
use cuprate_p2p::{block_downloader::BlockBatch, constants::LONG_BAN};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse, BlockchainWriteRequest},
    AltBlockInformation, HardFork, TransactionVerificationData, VerifiedBlockInformation,
};

use crate::{blockchain::types::ConsensusBlockchainReadHandle, signals::REORG_LOCK};

impl super::BlockchainManager {
    /// Handle an incoming [`Block`].
    ///
    /// This function will route to [`Self::handle_incoming_alt_block`] if the block does not follow
    /// the top of the main chain.
    ///
    /// Otherwise, this function will validate and add the block to the main chain.
    ///
    /// On success returns a [`bool`] indicating if the block was added to the main chain ([`true`])
    /// of an alt-chain ([`false`]).
    pub async fn handle_incoming_block(
        &mut self,
        block: Block,
        prepared_txs: HashMap<[u8; 32], TransactionVerificationData>,
    ) -> Result<bool, anyhow::Error> {
        if block.header.previous != self.cached_blockchain_context.top_hash {
            self.handle_incoming_alt_block(block, prepared_txs).await?;

            return Ok(false);
        }

        let VerifyBlockResponse::MainChain(verified_block) = self
            .block_verifier_service
            .ready()
            .await
            .expect("TODO")
            .call(VerifyBlockRequest::MainChain {
                block,
                prepared_txs,
            })
            .await?
        else {
            panic!("Incorrect response!");
        };

        self.add_valid_block_to_main_chain(verified_block).await;

        Ok(true)
    }

    /// Handle an incoming [`BlockBatch`].
    ///
    /// This function will route to [`Self::handle_incoming_block_batch_main_chain`] or [`Self::handle_incoming_block_batch_alt_chain`]
    /// depending on if the first block in the batch follows from the top of our chain.
    ///
    /// # Panics
    ///
    /// This function will panic if the batch is empty or if any internal service returns an unexpected
    /// error that we cannot recover from.
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
    /// recover from.
    async fn handle_incoming_block_batch_main_chain(&mut self, batch: BlockBatch) {
        info!(
            "Handling batch to main chain height: {}",
            batch.blocks.first().unwrap().0.number().unwrap()
        );

        let ban_cancel_download = || {
            batch.peer_handle.ban_peer(LONG_BAN);
            self.stop_current_block_downloader.notify_one();
        };

        let batch_prep_res = self
            .block_verifier_service
            .ready()
            .await
            .expect("TODO")
            .call(VerifyBlockRequest::MainChainBatchPrepareBlocks {
                blocks: batch.blocks,
            })
            .await;

        let prepped_blocks = match batch_prep_res {
            Ok(VerifyBlockResponse::MainChainBatchPrepped(prepped_blocks)) => prepped_blocks,
            Err(_) => {
                ban_cancel_download();
                return;
            }
            _ => panic!("Incorrect response!"),
        };

        for (block, txs) in prepped_blocks {
            let verify_res = self
                .block_verifier_service
                .ready()
                .await
                .expect("TODO")
                .call(VerifyBlockRequest::MainChainPrepped { block, txs })
                .await;

            let VerifyBlockResponse::MainChain(verified_block) = match verify_res {
                Ok(VerifyBlockResponse::MainChain(verified_block)) => verified_block,
                Err(_) => {
                    ban_cancel_download();
                    return;
                }
                _ => panic!("Incorrect response!"),
            };

            self.add_valid_block_to_main_chain(verified_block).await;
        }

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
    /// # Panics
    ///
    /// This function will panic if any internal service returns an unexpected error that we cannot
    /// recover from.
    async fn handle_incoming_block_batch_alt_chain(&mut self, batch: BlockBatch) {
        for (block, txs) in batch.blocks {
            // async blocks work as try blocks.
            let res = async {
                let txs = txs
                    .into_par_iter()
                    .map(|tx| {
                        let tx = new_tx_verification_data(tx)?;
                        Ok((tx.tx_hash, tx))
                    })
                    .collect::<Result<_, anyhow::Error>>()?;

                self.handle_incoming_alt_block(block, txs).await?;

                Ok(())
            }
            .await;

            if let Err(e) = res {
                batch.peer_handle.ban_peer(LONG_BAN);
                self.stop_current_block_downloader.notify_one();
                return;
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
    pub async fn handle_incoming_alt_block(
        &mut self,
        block: Block,
        prepared_txs: HashMap<[u8; 32], TransactionVerificationData>,
    ) -> Result<(), anyhow::Error> {
        let VerifyBlockResponse::AltChain(alt_block_info) = self
            .block_verifier_service
            .ready()
            .await
            .expect("TODO")
            .call(VerifyBlockRequest::AltChain {
                block,
                prepared_txs,
            })
            .await?
        else {
            panic!("Incorrect response!");
        };

        // TODO: check in consensus crate if alt block already exists.

        if alt_block_info.cumulative_difficulty
            > self.cached_blockchain_context.cumulative_difficulty
        {
            self.try_do_reorg(alt_block_info).await?;
            return Ok(());
        }

        self.blockchain_write_handle
            .ready()
            .await
            .expect("TODO")
            .call(BlockchainWriteRequest::WriteAltBlock(alt_block_info))
            .await?;

        Ok(())
    }

    /// Attempt a re-org with the given top block of the alt-chain.
    ///
    /// This function will take a write lock on [`REORG_LOCK`] and then set up the blockchain database
    /// and context cache to verify the alt-chain. It will then attempt to verify and add each block
    /// in the alt-chain to tha main-chain. Releasing the lock on [`REORG_LOCK`] when finished.
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
            .expect("TODO")
            .call(BlockchainReadRequest::AltBlocksInChain(
                top_alt_block.chain_id,
            ))
            .await?
        else {
            panic!("Incorrect response!");
        };

        alt_blocks.push(top_alt_block);

        let split_height = alt_blocks[0].height;
        let current_main_chain_height = self.cached_blockchain_context.chain_height;

        let BlockchainResponse::PopBlocks(old_main_chain_id) = self
            .blockchain_write_handle
            .ready()
            .await
            .expect("TODO")
            .call(BlockchainWriteRequest::PopBlocks(
                current_main_chain_height - split_height + 1,
            ))
            .await
            .expect("TODO")
        else {
            panic!("Incorrect response!");
        };

        self.blockchain_context_service
            .ready()
            .await
            .expect("TODO")
            .call(BlockChainContextRequest::PopBlocks {
                numb_blocks: current_main_chain_height - split_height + 1,
            })
            .await
            .expect("TODO");

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
    /// and context cache has had the top blocks popped to where the alt-chain meets the main-chain.
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
                .expect("TODO")
                .call(VerifyBlockRequest::MainChainPrepped {
                    block: prepped_block,
                    txs: prepped_txs,
                })
                .await?
            else {
                panic!("Incorrect response!");
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
        self.blockchain_context_service
            .ready()
            .await
            .expect("TODO")
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
            .expect("TODO");

        self.blockchain_write_handle
            .ready()
            .await
            .expect("TODO")
            .call(BlockchainWriteRequest::WriteBlock(verified_block))
            .await
            .expect("TODO");

        let BlockChainContextResponse::Context(blockchain_context) = self
            .blockchain_context_service
            .ready()
            .await
            .expect("TODO")
            .call(BlockChainContextRequest::GetContext)
            .await
            .expect("TODO")
        else {
            panic!("Incorrect response!");
        };

        self.cached_blockchain_context = blockchain_context.unchecked_blockchain_context().clone();
    }
}

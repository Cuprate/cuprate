use crate::blockchain::types::ConsensusBlockchainReadHandle;
use crate::signals::REORG_LOCK;
use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::block::PreparedBlock;
use cuprate_consensus::context::NewBlockData;
use cuprate_consensus::transactions::new_tx_verification_data;
use cuprate_consensus::{
    BlockChainContextRequest, BlockChainContextResponse, BlockVerifierService,
    ExtendedConsensusError, VerifyBlockRequest, VerifyBlockResponse, VerifyTxRequest,
    VerifyTxResponse,
};
use cuprate_p2p::block_downloader::BlockBatch;
use cuprate_types::blockchain::{
    BlockchainReadRequest, BlockchainResponse, BlockchainWriteRequest,
};
use cuprate_types::{
    AltBlockInformation, HardFork, TransactionVerificationData, VerifiedBlockInformation,
};
use futures::{TryFutureExt, TryStreamExt};
use monero_serai::block::Block;
use monero_serai::transaction::Transaction;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tower::{Service, ServiceExt};
use tracing::info;

impl super::BlockchainManager {
    pub async fn handle_incoming_block(
        &mut self,
        block: Block,
        prepared_txs: HashMap<[u8; 32], TransactionVerificationData>,
    ) -> Result<(), anyhow::Error> {
        if block.header.previous != self.cached_blockchain_context.top_hash {
            self.handle_incoming_alt_block(block, prepared_txs).await?;

            return Ok(());
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

        Ok(())
    }

    pub async fn handle_incoming_block_batch(&mut self, batch: BlockBatch) {
        let (first_block, _) = batch
            .blocks
            .first()
            .expect("Block batch should not be empty");

        if first_block.header.previous == self.cached_blockchain_context.top_hash {
            self.handle_incoming_block_batch_main_chain(batch)
                .await
                .expect("TODO");
        } else {
            self.handle_incoming_block_batch_alt_chain(batch)
                .await
                .expect("TODO");
        }
    }

    async fn handle_incoming_block_batch_main_chain(
        &mut self,
        batch: BlockBatch,
    ) -> Result<(), anyhow::Error> {
        info!(
            "Handling batch to main chain height: {}",
            batch.blocks.first().unwrap().0.number().unwrap()
        );

        let VerifyBlockResponse::MainChainBatchPrepped(prepped) = self
            .block_verifier_service
            .ready()
            .await
            .expect("TODO")
            .call(VerifyBlockRequest::MainChainBatchPrepareBlocks {
                blocks: batch.blocks,
            })
            .await?
        else {
            panic!("Incorrect response!");
        };

        for (block, txs) in prepped {
            let VerifyBlockResponse::MainChain(verified_block) = self
                .block_verifier_service
                .ready()
                .await
                .expect("TODO")
                .call(VerifyBlockRequest::MainChainPrepped { block, txs })
                .await?
            else {
                panic!("Incorrect response!");
            };

            self.add_valid_block_to_main_chain(verified_block).await;
        }

        Ok(())
    }

    async fn handle_incoming_block_batch_alt_chain(
        &mut self,
        batch: BlockBatch,
    ) -> Result<(), anyhow::Error> {
        for (block, txs) in batch.blocks {
            let txs = txs
                .into_par_iter()
                .map(|tx| {
                    let tx = new_tx_verification_data(tx)?;
                    Ok((tx.tx_hash, tx))
                })
                .collect::<Result<_, anyhow::Error>>()?;

            self.handle_incoming_alt_block(block, txs).await?;
        }

        Ok(())
    }

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
            // TODO: ban the peer if the reorg failed.

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

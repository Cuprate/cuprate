mod batch_handler;
mod handler;

use crate::blockchain::types::ConsensusBlockchainReadHandle;
use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::context::RawBlockChainContext;
use cuprate_consensus::{
    BlockChainContextRequest, BlockChainContextResponse, BlockChainContextService,
    BlockVerifierService, ExtendedConsensusError, TxVerifierService, VerifyBlockRequest,
    VerifyBlockResponse, VerifyTxRequest, VerifyTxResponse,
};
use cuprate_p2p::block_downloader::BlockBatch;
use cuprate_types::blockchain::{BlockchainReadRequest, BlockchainResponse};
use cuprate_types::Chain;
use futures::StreamExt;
use tokio::sync::mpsc::Receiver;
use tower::{Service, ServiceExt};
use tracing::error;

pub struct BlockchainManager {
    blockchain_write_handle: BlockchainWriteHandle,
    blockchain_read_handle: BlockchainReadHandle,
    blockchain_context_service: BlockChainContextService,
    cached_blockchain_context: RawBlockChainContext,
    block_verifier_service: BlockVerifierService<
        BlockChainContextService,
        TxVerifierService<ConsensusBlockchainReadHandle>,
        ConsensusBlockchainReadHandle,
    >,
}

impl BlockchainManager {
    pub const fn new(
        blockchain_write_handle: BlockchainWriteHandle,
        blockchain_read_handle: BlockchainReadHandle,
        blockchain_context_service: BlockChainContextService,
        block_verifier_service: BlockVerifierService<
            BlockChainContextService,
            TxVerifierService<ConsensusBlockchainReadHandle>,
            ConsensusBlockchainReadHandle,
        >,
    ) -> Self {
        Self {
            blockchain_write_handle,
            blockchain_read_handle,
            blockchain_context_service,
            cached_blockchain_context: todo!(),
            block_verifier_service,
        }
    }

    async fn handle_incoming_main_chain_batch(
        &mut self,
        batch: BlockBatch,
    ) -> Result<(), anyhow::Error> {
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
            let VerifyBlockResponse::MainChain(verified_block) = block_verifier_service
                .ready()
                .await
                .expect("TODO")
                .call(VerifyBlockRequest::MainChainPrepped { block, txs })
                .await
                .unwrap()
            else {
                panic!("Incorrect response!");
            };

            blockchain_context_service
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

            blockchain_write_handle
                .ready()
                .await
                .expect("TODO")
                .call(BlockchainWriteRequest::WriteBlock(verified_block))
                .await
                .expect("TODO");
        }
    }

    async fn handle_incoming_block_batch(&mut self, batch: BlockBatch) {
        let (first_block, _) = batch
            .blocks
            .first()
            .expect("Block batch should not be empty");

        if first_block.header.previous == self.cached_blockchain_context.top_hash {
            todo!("Main chain")
        } else {
            todo!("Alt chain")
        }
    }

    pub async fn run(mut self, mut batch_rx: Receiver<BlockBatch>) {
        loop {
            tokio::select! {
                Some(batch) = batch_rx.recv() => {
                    self.handle_incoming_block_batch(
                        batch,
                    ).await;
                }
                else => {
                    todo!("TODO: exit the BC manager")
                }
            }
        }
    }
}

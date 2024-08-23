mod batch_handler;

use crate::blockchain::manager::batch_handler::handle_incoming_block_batch;
use crate::blockchain::types::ConsensusBlockchainReadHandle;
use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::{BlockChainContextService, BlockVerifierService, TxVerifierService};
use cuprate_p2p::block_downloader::BlockBatch;
use futures::StreamExt;
use tokio::sync::mpsc::Receiver;

pub struct BlockchainManager {
    blockchain_write_handle: BlockchainWriteHandle,
    blockchain_read_handle: BlockchainReadHandle,
    blockchain_context_service: BlockChainContextService,
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
            block_verifier_service,
        }
    }

    pub async fn run(mut self, mut batch_rx: Receiver<BlockBatch>) {
        loop {
            tokio::select! {
                Some(batch) = batch_rx.recv() => {
                    handle_incoming_block_batch(
                        batch,
                        &mut self.block_verifier_service,
                        &mut self.blockchain_context_service,
                        &mut self.blockchain_read_handle,
                        &mut self.blockchain_write_handle
                    ).await;
                }
                else => {
                    todo!("TODO: exit the BC manager")
                }
            }
        }
    }
}

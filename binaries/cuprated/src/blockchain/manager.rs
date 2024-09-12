mod handler;

use std::collections::HashMap;
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
use cuprate_types::{Chain, TransactionVerificationData};
use futures::StreamExt;
use monero_serai::block::Block;
use tokio::sync::mpsc;
use tokio::sync::{Notify, oneshot};
use tower::{Service, ServiceExt};
use tracing::error;

pub struct IncomingBlock {
    block: Block,
    prepped_txs: HashMap<[u8; 32], TransactionVerificationData>,
    response_tx: oneshot::Sender<Result<(), anyhow::Error>>,
}

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

    // TODO: stop_current_block_downloader: Notify,
}

impl BlockchainManager {
    pub async fn new(
        blockchain_write_handle: BlockchainWriteHandle,
        blockchain_read_handle: BlockchainReadHandle,
        mut blockchain_context_service: BlockChainContextService,
        block_verifier_service: BlockVerifierService<
            BlockChainContextService,
            TxVerifierService<ConsensusBlockchainReadHandle>,
            ConsensusBlockchainReadHandle,
        >,
    ) -> Self {
        let BlockChainContextResponse::Context(blockchain_context) = blockchain_context_service
            .ready()
            .await
            .expect("TODO")
            .call(BlockChainContextRequest::GetContext)
            .await
            .expect("TODO") else {
            panic!("Blockchain context service returned wrong response!");
        };

        Self {
            blockchain_write_handle,
            blockchain_read_handle,
            blockchain_context_service,
            cached_blockchain_context: blockchain_context.unchecked_blockchain_context().clone(),
            block_verifier_service,
        }
    }

    pub async fn run(mut self, mut block_batch_rx: mpsc::Receiver<BlockBatch>, mut block_single_rx: mpsc::Receiver<IncomingBlock>) {
        loop {
            tokio::select! {
                Some(batch) = block_batch_rx.recv() => {
                    self.handle_incoming_block_batch(
                        batch,
                    ).await;
                }
                Some(incoming_block) = block_single_rx.recv() => {
                    let IncomingBlock {
                        block,
                        prepped_txs,
                        response_tx
                    } = incoming_block;

                    let res = self.handle_incoming_block(block, prepped_txs).await;
                    let _ = response_tx.send(res);
                }
                else => {
                    todo!("TODO: exit the BC manager")
                }
            }
        }
    }
}

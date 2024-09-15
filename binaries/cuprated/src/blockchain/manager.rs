pub(super) mod commands;
mod handler;

use crate::blockchain::manager::commands::BlockchainManagerCommand;
use crate::blockchain::types::ConsensusBlockchainReadHandle;
use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::context::RawBlockChainContext;
use cuprate_consensus::{
    BlockChainContextRequest, BlockChainContextResponse, BlockChainContextService,
    BlockVerifierService, ExtendedConsensusError, TxVerifierService, VerifyBlockRequest,
    VerifyBlockResponse, VerifyTxRequest, VerifyTxResponse,
};
use cuprate_p2p::block_downloader::BlockBatch;
use cuprate_p2p::BroadcastSvc;
use cuprate_p2p_core::ClearNet;
use cuprate_types::blockchain::{BlockchainReadRequest, BlockchainResponse};
use cuprate_types::{Chain, TransactionVerificationData};
use futures::StreamExt;
use monero_serai::block::Block;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::{oneshot, Notify};
use tower::{Service, ServiceExt};
use tracing::error;
use tracing_subscriber::fmt::time::FormatTime;

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
    stop_current_block_downloader: Arc<Notify>,
    broadcast_svc: BroadcastSvc<ClearNet>,
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
        stop_current_block_downloader: Arc<Notify>,
        broadcast_svc: BroadcastSvc<ClearNet>,
    ) -> Self {
        let BlockChainContextResponse::Context(blockchain_context) = blockchain_context_service
            .ready()
            .await
            .expect("TODO")
            .call(BlockChainContextRequest::GetContext)
            .await
            .expect("TODO")
        else {
            panic!("Blockchain context service returned wrong response!");
        };

        Self {
            blockchain_write_handle,
            blockchain_read_handle,
            blockchain_context_service,
            cached_blockchain_context: blockchain_context.unchecked_blockchain_context().clone(),
            block_verifier_service,
            stop_current_block_downloader,
            broadcast_svc,
        }
    }

    pub async fn run(
        mut self,
        mut block_batch_rx: mpsc::Receiver<BlockBatch>,
        mut command_rx: mpsc::Receiver<BlockchainManagerCommand>,
    ) {
        loop {
            tokio::select! {
                Some(batch) = block_batch_rx.recv() => {
                    self.handle_incoming_block_batch(
                        batch,
                    ).await;
                }
                Some(incoming_command) = command_rx.recv() => {
                    self.handle_command(incoming_command).await;
                }
                else => {
                    todo!("TODO: exit the BC manager")
                }
            }
        }
    }
}

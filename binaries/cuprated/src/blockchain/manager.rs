pub(super) mod commands;
mod handler;

use crate::blockchain::interface::INCOMING_BLOCK_TX;
use crate::blockchain::manager::commands::BlockchainManagerCommand;
use crate::blockchain::types::ChainService;
use crate::blockchain::{
    syncer,
    types::{ConcreteBlockVerifierService, ConsensusBlockchainReadHandle},
};
use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::context::RawBlockChainContext;
use cuprate_consensus::{
    BlockChainContextRequest, BlockChainContextResponse, BlockChainContextService,
    BlockVerifierService, ExtendedConsensusError, TxVerifierService, VerifyBlockRequest,
    VerifyBlockResponse, VerifyTxRequest, VerifyTxResponse,
};
use cuprate_p2p::block_downloader::{BlockBatch, BlockDownloaderConfig};
use cuprate_p2p::{BroadcastSvc, NetworkInterface};
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

pub async fn init_blockchain_manger(
    clearnet_interface: NetworkInterface<ClearNet>,
    blockchain_write_handle: BlockchainWriteHandle,
    blockchain_read_handle: BlockchainReadHandle,
    mut blockchain_context_service: BlockChainContextService,
    block_verifier_service: ConcreteBlockVerifierService,
    block_downloader_config: BlockDownloaderConfig,
) {
    let (batch_tx, batch_rx) = mpsc::channel(1);
    let stop_current_block_downloader = Arc::new(Notify::new());
    let (command_tx, command_rx) = mpsc::channel(1);

    INCOMING_BLOCK_TX.set(command_tx).unwrap();

    tokio::spawn(syncer::syncer(
        blockchain_context_service.clone(),
        ChainService(blockchain_read_handle.clone()),
        clearnet_interface.clone(),
        batch_tx,
        stop_current_block_downloader.clone(),
        block_downloader_config,
    ));

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

    let manger = BlockchainManager {
        blockchain_write_handle,
        blockchain_read_handle,
        blockchain_context_service,
        cached_blockchain_context: blockchain_context.unchecked_blockchain_context().clone(),
        block_verifier_service,
        stop_current_block_downloader,
        broadcast_svc,
    };

    tokio::spawn(manger.run(batch_rx, command_rx));
}

/// The blockchain manager.
///
/// This handles all mutation of the blockchain, anything that changes the state of the blockchain must
/// go through this.
///
/// Other parts of Cuprate can interface with this by using the functions in [`interface`](super::interface).
pub struct BlockchainManager {
    /// The [`BlockchainWriteHandle`], this is the _only_ part of Cuprate where a [`BlockchainWriteHandle`]
    /// is held.
    blockchain_write_handle: BlockchainWriteHandle,
    /// A [`BlockchainReadHandle`].
    blockchain_read_handle: BlockchainReadHandle,
    // TODO: Improve the API of the cache service.
    // TODO: rename the cache service -> `BlockchainContextService`.
    /// The blockchain context cache, this caches the current state of the blockchain to quickly calculate/retrieve
    /// values without needing to go to a [`BlockchainReadHandle`].
    blockchain_context_service: BlockChainContextService,
    /// A cached context representing the current state.
    cached_blockchain_context: RawBlockChainContext,
    /// The block verifier service, to verify incoming blocks.
    block_verifier_service: BlockVerifierService<
        BlockChainContextService,
        TxVerifierService<ConsensusBlockchainReadHandle>,
        ConsensusBlockchainReadHandle,
    >,
    /// A [`Notify`] to tell the [syncer](syncer::syncer) that we want to cancel this current download
    /// attempt.
    stop_current_block_downloader: Arc<Notify>,
    /// The broadcast service, to broadcast new blocks.
    broadcast_svc: BroadcastSvc<ClearNet>,
}

impl BlockchainManager {
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

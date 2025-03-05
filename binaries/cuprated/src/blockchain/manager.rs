use std::{collections::HashMap, sync::Arc};

use futures::StreamExt;
use monero_serai::block::Block;
use tokio::sync::{mpsc, oneshot, Notify};
use tower::{BoxError, Service, ServiceExt};
use tracing::error;

use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::{
    BlockChainContextRequest, BlockChainContextResponse, BlockchainContextService,
    ExtendedConsensusError,
};
use cuprate_p2p::{
    block_downloader::{BlockBatch, BlockDownloaderConfig},
    BroadcastSvc, NetworkInterface,
};
use cuprate_p2p_core::ClearNet;
use cuprate_txpool::service::TxpoolWriteHandle;
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    Chain, TransactionVerificationData,
};

use crate::{
    blockchain::{
        chain_service::ChainService, interface::COMMAND_TX, syncer,
        types::ConsensusBlockchainReadHandle,
    },
    constants::PANIC_CRITICAL_SERVICE_ERROR,
};

mod commands;
mod handler;

pub use commands::{BlockchainManagerCommand, IncomingBlockOk};

/// Initialize the blockchain manager.
///
/// This function sets up the [`BlockchainManager`] and the [`syncer`] so that the functions in [`interface`](super::interface)
/// can be called.
pub async fn init_blockchain_manager(
    clearnet_interface: NetworkInterface<ClearNet>,
    blockchain_write_handle: BlockchainWriteHandle,
    blockchain_read_handle: BlockchainReadHandle,
    txpool_write_handle: TxpoolWriteHandle,
    mut blockchain_context_service: BlockchainContextService,
    block_downloader_config: BlockDownloaderConfig,
) {
    // TODO: find good values for these size limits
    let (batch_tx, batch_rx) = mpsc::channel(1);
    let stop_current_block_downloader = Arc::new(Notify::new());
    let (command_tx, command_rx) = mpsc::channel(3);

    COMMAND_TX.set(command_tx).unwrap();

    tokio::spawn(syncer::syncer(
        blockchain_context_service.clone(),
        ChainService(blockchain_read_handle.clone()),
        clearnet_interface.clone(),
        batch_tx,
        Arc::clone(&stop_current_block_downloader),
        block_downloader_config,
    ));

    let manager = BlockchainManager {
        blockchain_write_handle,
        blockchain_read_handle: ConsensusBlockchainReadHandle::new(
            blockchain_read_handle,
            BoxError::from,
        ),
        txpool_write_handle,
        blockchain_context_service,
        stop_current_block_downloader,
        broadcast_svc: clearnet_interface.broadcast_svc(),
    };

    tokio::spawn(manager.run(batch_rx, command_rx));
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
    blockchain_read_handle: ConsensusBlockchainReadHandle,
    /// A [`TxpoolWriteHandle`].
    txpool_write_handle: TxpoolWriteHandle,
    /// The blockchain context cache, this caches the current state of the blockchain to quickly calculate/retrieve
    /// values without needing to go to a [`BlockchainReadHandle`].
    blockchain_context_service: BlockchainContextService,
    /// A [`Notify`] to tell the [syncer](syncer::syncer) that we want to cancel this current download
    /// attempt.
    stop_current_block_downloader: Arc<Notify>,
    /// The broadcast service, to broadcast new blocks.
    broadcast_svc: BroadcastSvc<ClearNet>,
}

impl BlockchainManager {
    /// The [`BlockchainManager`] task.
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

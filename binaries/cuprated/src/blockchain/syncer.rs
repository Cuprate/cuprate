use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use futures::{FutureExt, StreamExt};
use tokio::sync::{futures::Notified, mpsc, watch, Notify, OwnedSemaphorePermit, Semaphore};
use tower::{Service, ServiceExt};
use tracing::instrument;

use cuprate_consensus::{BlockChainContextRequest, BlockChainContextResponse, BlockchainContext};
use cuprate_consensus_context::BlockchainContextService;
use cuprate_p2p::{
    block_downloader::{BlockBatch, BlockDownloaderConfig, ChainSvcRequest, ChainSvcResponse},
    NetworkInterface, PeerSetRequest, PeerSetResponse,
};
use cuprate_p2p_core::{ClearNet, NetworkZone};
use cuprate_wire::CoreSyncData;

use crate::blockchain::interface::is_block_being_handled;

/// An error returned from the [`syncer`].
#[derive(Debug, thiserror::Error)]
pub enum SyncerError {
    #[error("Incoming block channel closed.")]
    IncomingBlockChannelClosed,
    #[error("One of our services returned an error: {0}.")]
    ServiceError(#[from] tower::BoxError),
}

/// The syncer tasks that makes sure we are fully synchronised with our connected peers.
#[instrument(level = "debug", skip_all)]
#[expect(clippy::significant_drop_tightening)]
pub async fn syncer<CN>(
    mut context_svc: BlockchainContextService,
    our_chain: CN,
    mut clearnet_interface: NetworkInterface<ClearNet>,
    incoming_block_batch_tx: mpsc::Sender<(BlockBatch, Arc<OwnedSemaphorePermit>)>,
    stop_current_block_downloader: Arc<Notify>,
    block_downloader_config: BlockDownloaderConfig,
    mut syncer_handle: SyncerHandle,
) -> Result<(), SyncerError>
where
    CN: Service<
            ChainSvcRequest<ClearNet>,
            Response = ChainSvcResponse<ClearNet>,
            Error = tower::BoxError,
        > + Clone
        + Send
        + 'static,
    CN::Future: Send + 'static,
{
    let semaphore = Arc::new(Semaphore::new(1));
    let mut sync_permit = Arc::new(Arc::clone(&semaphore).acquire_owned().await.unwrap());

    tracing::info!("Starting blockchain syncer");

    loop {
        syncer_handle.notify_syncer.notified().await;

        let blockchain_context = context_svc.blockchain_context();

        match check_sync_status(blockchain_context, &mut clearnet_interface).await? {
            SyncStatus::BehindPeers => {}
            SyncStatus::NoPeers => continue,
            SyncStatus::Synced => {
                if let Some(synced) = syncer_handle.synced_tx.take() {
                    #[expect(clippy::let_underscore_must_use)]
                    let _ = synced.send(());
                }
                continue;
            }
        }

        tracing::debug!(
            "We are behind peers claimed cumulative difficulty, starting block downloader"
        );

        let mut block_batch_stream =
            clearnet_interface.block_downloader(our_chain.clone(), block_downloader_config);

        loop {
            tokio::select! {
                () = stop_current_block_downloader.notified() => {
                    tracing::info!("Received stop signal, stopping block downloader");

                    drop(sync_permit);
                    sync_permit = Arc::new(Arc::clone(&semaphore).acquire_owned().await.unwrap());

                    break;
                }
                batch = block_batch_stream.next() => {
                    let Some(batch) = batch else {
                        // Wait for all references to the permit have been dropped (which means all blocks in the queue
                        // have been handled before checking if we are synced.
                        drop(sync_permit);
                        sync_permit = Arc::new(Arc::clone(&semaphore).acquire_owned().await.unwrap());

                        let blockchain_context = context_svc.blockchain_context();

                        if check_sync_status(blockchain_context, &mut clearnet_interface).await? == SyncStatus::Synced {
                            tracing::info!("Synchronised with the network.");
                            if let Some(synced) = syncer_handle.synced_tx.take() {
                                #[expect(clippy::let_underscore_must_use)]
                                let _ = synced.send(());
                            }
                        }

                        break;
                    };

                    tracing::debug!("Got batch, len: {}", batch.blocks.len());
                    if incoming_block_batch_tx.send((batch, Arc::clone(&sync_permit))).await.is_err() {
                        return Err(SyncerError::IncomingBlockChannelClosed);
                    }
                }
            }
        }
    }
}

#[derive(Debug, PartialEq)]
enum SyncStatus {
    NoPeers,
    BehindPeers,
    Synced,
}

/// Checks if we are behind the connected peers.
async fn check_sync_status(
    blockchain_context: &BlockchainContext,
    mut clearnet_interface: &mut NetworkInterface<ClearNet>,
) -> Result<SyncStatus, tower::BoxError> {
    let PeerSetResponse::MostPoWSeen {
        cumulative_difficulty,
        ..
    } = clearnet_interface
        .peer_set()
        .ready()
        .await?
        .call(PeerSetRequest::MostPoWSeen)
        .await?
    else {
        unreachable!();
    };

    if cumulative_difficulty == 0 {
        return Ok(SyncStatus::NoPeers);
    }

    if cumulative_difficulty > blockchain_context.cumulative_difficulty {
        return Ok(SyncStatus::BehindPeers);
    }

    Ok(SyncStatus::Synced)
}

/// The handle for the blockchain syncer.
pub struct SyncerHandle {
    /// The syncer notify channel, used to wake the syncer.
    notify_syncer: Arc<Notify>,
    /// The synced notify channel, used to wake the tasks waiting on cuprate to be synced.
    synced_tx: Option<futures::channel::oneshot::Sender<()>>,
}

/// Notifications for sync state.
#[derive(Clone)]
pub struct SyncNotify {
    /// The syncer notify channel, used to wake the syncer.
    notify_syncer: Arc<Notify>,
    /// The synced notify channel, used to wake the tasks waiting on cuprate to be synced.
    synced: futures::future::Shared<futures::channel::oneshot::Receiver<()>>,
}

impl SyncNotify {
    /// Creates a new [`SyncNotify`] with the corresponding handle for the syncer.
    pub fn new() -> (Self, SyncerHandle) {
        let (synced_tx, rx) = futures::channel::oneshot::channel();

        let notify_syncer = Arc::new(Notify::new());

        (
            Self {
                notify_syncer: Arc::clone(&notify_syncer),
                synced: rx.shared(),
            },
            SyncerHandle {
                notify_syncer,
                synced_tx: Some(synced_tx),
            },
        )
    }

    /// Checks if any tasks need to be waked for this incoming [`CoreSyncData`] and wakes them.
    pub fn check_wake_waiters(
        &self,
        our_cumulative_difficulty: u128,
        our_height: usize,
        peer_csd: &CoreSyncData,
    ) {
        // If we are synced and the syncer hasn't yet set the node to synced, wake the syncer.
        if our_cumulative_difficulty == peer_csd.cumulative_difficulty()
            && self.synced.peek().is_none()
        {
            self.notify_syncer.notify_one();
        }

        // If we are behind the peer, and we aren't just one block behind with the blockchain manager handling the block, wake the syncer.
        if peer_csd.cumulative_difficulty() > our_cumulative_difficulty
            && !(peer_csd.current_height - 1 == our_height as u64
                && is_block_being_handled(&peer_csd.top_id))
        {
            self.notify_syncer.notify_one();
        }
    }

    /// A future that resolves when cuprate has synced with the network.
    pub fn wait_for_synced(
        &self,
    ) -> impl Future<Output = Result<(), futures::channel::oneshot::Canceled>> + 'static {
        self.synced.clone()
    }
}

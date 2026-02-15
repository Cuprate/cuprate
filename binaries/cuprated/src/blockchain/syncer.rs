// FIXME: This whole module is not great and should be rewritten when the PeerSet is made.
use std::sync::Arc;

use futures::StreamExt;
use tokio::sync::{mpsc, Notify, OwnedSemaphorePermit, Semaphore};
use tower::{Service, ServiceExt};
use tracing::instrument;

use cuprate_consensus::{BlockChainContextRequest, BlockChainContextResponse, BlockchainContext};
use cuprate_consensus_context::BlockchainContextService;
use cuprate_p2p::{
    block_downloader::{BlockBatch, BlockDownloaderConfig, ChainSvcRequest, ChainSvcResponse},
    NetworkInterface, PeerSetRequest, PeerSetResponse,
};
use cuprate_p2p_core::{client::PeerSyncCallback, ClearNet, NetworkZone};

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
#[expect(clippy::significant_drop_tightening, clippy::too_many_arguments)]
pub async fn syncer<CN>(
    mut context_svc: BlockchainContextService,
    our_chain: CN,
    mut clearnet_interface: NetworkInterface<ClearNet>,
    incoming_block_batch_tx: mpsc::Sender<(BlockBatch, Arc<OwnedSemaphorePermit>)>,
    stop_current_block_downloader: Arc<Notify>,
    block_downloader_config: BlockDownloaderConfig,
    synced_notify: Arc<Notify>,
    peer_sync_callback: PeerSyncCallback,
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

    let mut initial_sync = true;

    'outer: loop {
        tracing::trace!("Checking connected peers to see if we are behind.");

        // Wait until we are behind peers before starting the block downloader.
        match check_sync_status(context_svc.blockchain_context(), &mut clearnet_interface).await? {
            SyncStatus::BehindPeers => {}
            SyncStatus::Synced => {
                if initial_sync {
                    tracing::info!("Synchronised with the network.");
                    synced_notify.notify_one();
                    initial_sync = false;
                }
                tracing::debug!("Waiting for new sync info.");
                peer_sync_callback.notified().await;
                continue;
            }
            SyncStatus::NoPeers => {
                tracing::debug!("Waiting for peers to connect.");
                peer_sync_callback.wake_on_first_peers_arm();
                peer_sync_callback.notified().await;
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

                    continue 'outer;
                }
                batch = block_batch_stream.next() => {
                    let Some(batch) = batch else {
                        // Wait for all references to the permit have been dropped (which means all blocks in the queue
                        // have been handled before checking if we are synced.
                        drop(sync_permit);
                        sync_permit = Arc::new(Arc::clone(&semaphore).acquire_owned().await.unwrap());

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

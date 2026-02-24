// FIXME: This whole module is not great and should be rewritten when the PeerSet is made.
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use futures::StreamExt;
use tokio::{
    sync::{mpsc, Notify, OwnedSemaphorePermit, Semaphore},
    time::{interval, Instant},
};
use tower::{Service, ServiceExt};
use tracing::instrument;

use cuprate_consensus::{BlockChainContextRequest, BlockChainContextResponse, BlockchainContext};
use cuprate_consensus_context::BlockchainContextService;
use cuprate_p2p::{
    block_downloader::{BlockBatch, BlockDownloaderConfig, ChainSvcRequest, ChainSvcResponse},
    NetworkInterface, PeerSetRequest, PeerSetResponse,
};
use cuprate_p2p_core::{ClearNet, NetworkZone};

const CHECK_SYNC_FREQUENCY: Duration = Duration::from_secs(30);

/// Returns a formatted string showing the sync progress, or empty string if not syncing.
#[expect(clippy::cast_precision_loss)]
pub fn format_sync_progress(sync_target_height: &AtomicUsize, chain_height: usize) -> String {
    let target = sync_target_height.load(Ordering::Relaxed);
    if target == 0 {
        return String::new();
    }

    let percent = f64::min((chain_height as f64 / target as f64) * 100.0, 99.99);
    let left = target.saturating_sub(chain_height);
    format!(" ({percent:.2}%, {left} left)")
}

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
    sync_target_height: Arc<AtomicUsize>,
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
    tracing::info!("Starting blockchain syncer");

    let mut check_sync_interval = interval(CHECK_SYNC_FREQUENCY);

    tracing::debug!("Waiting for new sync info in top sync channel");

    let semaphore = Arc::new(Semaphore::new(1));

    let mut sync_permit = Arc::new(Arc::clone(&semaphore).acquire_owned().await.unwrap());
    loop {
        check_sync_interval.tick().await;

        tracing::trace!("Checking connected peers to see if we are behind",);

        let blockchain_context = context_svc.blockchain_context();

        match check_sync_status(blockchain_context, &mut clearnet_interface).await? {
            SyncStatus::BehindPeers => {}
            SyncStatus::NoPeers => continue,
            SyncStatus::Synced => {
                synced_notify.notify_one();
                continue;
            }
        }

        tracing::debug!(
            "We are behind peers claimed cumulative difficulty, starting block downloader"
        );
        let mut block_batch_stream =
            clearnet_interface.block_downloader(our_chain.clone(), block_downloader_config);

        let (_, initial_target) = network_tip(&mut clearnet_interface).await?;
        sync_target_height.store(initial_target, Ordering::Relaxed);

        let mut last_target_refresh = Instant::now();
        loop {
            tokio::select! {
                () = stop_current_block_downloader.notified() => {
                    tracing::info!("Received stop signal, stopping block downloader");

                    drop(sync_permit);
                    sync_permit = Arc::new(Arc::clone(&semaphore).acquire_owned().await.unwrap());

                    sync_target_height.store(0, Ordering::Relaxed);

                    break;
                }
                batch = block_batch_stream.next() => {
                    let Some(batch) = batch else {
                        // Wait for all references to the permit have been dropped (which means all blocks in the queue
                        // have been handled before checking if we are synced.
                        drop(sync_permit);
                        sync_permit = Arc::new(Arc::clone(&semaphore).acquire_owned().await.unwrap());

                        sync_target_height.store(0, Ordering::Relaxed);

                        let blockchain_context = context_svc.blockchain_context();

                        if check_sync_status(blockchain_context, &mut clearnet_interface).await? == SyncStatus::Synced {
                            tracing::info!("Synchronised with the network.");
                            synced_notify.notify_one();
                        }

                        break;
                    };

                    tracing::debug!("Got batch, len: {}", batch.blocks.len());
                    if incoming_block_batch_tx.send((batch, Arc::clone(&sync_permit))).await.is_err() {
                        return Err(SyncerError::IncomingBlockChannelClosed);
                    }

                    if last_target_refresh.elapsed() >= Duration::from_secs(10) {
                        last_target_refresh = Instant::now();
                        let (_, target_height) = network_tip(&mut clearnet_interface).await?;
                        sync_target_height.store(target_height, Ordering::Relaxed);
                    }
                }
            }
        }
    }
}

/// Returns relevant information about the current network tip.
async fn network_tip(
    clearnet_interface: &mut NetworkInterface<ClearNet>,
) -> Result<(u128, usize), tower::BoxError> {
    let PeerSetResponse::MostPoWSeen {
        cumulative_difficulty,
        height,
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

    Ok((cumulative_difficulty, height))
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
    clearnet_interface: &mut NetworkInterface<ClearNet>,
) -> Result<SyncStatus, tower::BoxError> {
    let (cumulative_difficulty, _) = network_tip(clearnet_interface).await?;

    if cumulative_difficulty == 0 {
        return Ok(SyncStatus::NoPeers);
    }

    if cumulative_difficulty > blockchain_context.cumulative_difficulty {
        return Ok(SyncStatus::BehindPeers);
    }

    Ok(SyncStatus::Synced)
}

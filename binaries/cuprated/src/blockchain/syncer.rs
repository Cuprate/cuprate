// FIXME: This whole module is not great and should be rewritten when the PeerSet is made.
use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use tokio::{
    sync::{mpsc, oneshot, Notify},
    time::interval,
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
pub async fn syncer<CN>(
    mut context_svc: BlockchainContextService,
    our_chain: CN,
    mut clearnet_interface: NetworkInterface<ClearNet>,
    incoming_block_batch_tx: mpsc::Sender<BlockBatch>,
    stop_current_block_downloader: Arc<Notify>,
    block_downloader_config: BlockDownloaderConfig,
) -> Result<(), SyncerError>
where
    CN: Service<ChainSvcRequest<ClearNet>, Response = ChainSvcResponse<ClearNet>, Error = tower::BoxError>
        + Clone
        + Send
        + 'static,
    CN::Future: Send + 'static,
{
    tracing::info!("Starting blockchain syncer");

    let mut check_sync_interval = interval(CHECK_SYNC_FREQUENCY);

    tracing::debug!("Waiting for new sync info in top sync channel");

    loop {
        check_sync_interval.tick().await;

        tracing::trace!("Checking connected peers to see if we are behind",);

        let blockchain_context = context_svc.blockchain_context();

        if !check_behind_peers(blockchain_context, &mut clearnet_interface).await? {
            continue;
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
                    break;
                }
                batch = block_batch_stream.next() => {
                    let Some(batch) = batch else {
                        let blockchain_context = context_svc.blockchain_context();

                        if !check_behind_peers(blockchain_context, &mut clearnet_interface).await? {
                            tracing::info!("Synchronised with the network.");
                        }

                        break;
                    };

                    tracing::debug!("Got batch, len: {}", batch.blocks.len());
                    if incoming_block_batch_tx.send(batch).await.is_err() {
                        return Err(SyncerError::IncomingBlockChannelClosed);
                    }
                }
            }
        }
    }
}

/// Returns `true` if we are behind the current connected network peers.
async fn check_behind_peers(
    blockchain_context: &BlockchainContext,
    mut clearnet_interface: &mut NetworkInterface<ClearNet>,
) -> Result<bool, tower::BoxError> {
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

    if cumulative_difficulty <= blockchain_context.cumulative_difficulty {
        return Ok(false);
    }

    Ok(true)
}

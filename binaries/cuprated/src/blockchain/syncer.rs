// FIXME: This whole module is not great and should be rewritten when the PeerSet is made.
use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use tokio::{
    sync::{mpsc, Notify},
    time::interval,
};
use tower::{Service, ServiceExt};
use tracing::instrument;

use cuprate_consensus::{BlockChainContext, BlockChainContextRequest, BlockChainContextResponse};
use cuprate_p2p::{
    block_downloader::{BlockBatch, BlockDownloaderConfig, ChainSvcRequest, ChainSvcResponse},
    NetworkInterface, PeerSetRequest, PeerSetResponse,
};
use cuprate_p2p_core::ClearNet;

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
pub async fn syncer<C, CN>(
    mut context_svc: C,
    our_chain: CN,
    mut clearnet_interface: NetworkInterface<ClearNet>,
    incoming_block_batch_tx: mpsc::Sender<BlockBatch>,
    stop_current_block_downloader: Arc<Notify>,
    block_downloader_config: BlockDownloaderConfig,
) -> Result<(), SyncerError>
where
    C: Service<
        BlockChainContextRequest,
        Response = BlockChainContextResponse,
        Error = tower::BoxError,
    >,
    C::Future: Send + 'static,
    CN: Service<ChainSvcRequest, Response = ChainSvcResponse, Error = tower::BoxError>
        + Clone
        + Send
        + 'static,
    CN::Future: Send + 'static,
{
    tracing::info!("Starting blockchain syncer");

    let mut check_sync_interval = interval(CHECK_SYNC_FREQUENCY);

    let BlockChainContextResponse::Context(mut blockchain_ctx) = context_svc
        .ready()
        .await?
        .call(BlockChainContextRequest::Context)
        .await?
    else {
        unreachable!();
    };

    tracing::debug!("Waiting for new sync info in top sync channel");

    loop {
        check_sync_interval.tick().await;

        tracing::trace!("Checking connected peers to see if we are behind",);

        check_update_blockchain_context(&mut context_svc, &mut blockchain_ctx).await?;
        let raw_blockchain_context = blockchain_ctx.unchecked_blockchain_context();

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

        if cumulative_difficulty <= raw_blockchain_context.cumulative_difficulty {
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
                    tracing::info!("Stopping block downloader");
                    break;
                }
                batch = block_batch_stream.next() => {
                    let Some(batch) = batch else {
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

/// Checks if we should update the given [`BlockChainContext`] and updates it if needed.
async fn check_update_blockchain_context<C>(
    context_svc: C,
    old_context: &mut BlockChainContext,
) -> Result<(), tower::BoxError>
where
    C: Service<
        BlockChainContextRequest,
        Response = BlockChainContextResponse,
        Error = tower::BoxError,
    >,
    C::Future: Send + 'static,
{
    if old_context.blockchain_context().is_ok() {
        return Ok(());
    }

    let BlockChainContextResponse::Context(ctx) = context_svc
        .oneshot(BlockChainContextRequest::Context)
        .await?
    else {
        unreachable!();
    };

    *old_context = ctx;

    Ok(())
}

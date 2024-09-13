use std::pin::pin;
use std::time::Duration;

use futures::StreamExt;
use tokio::{
    sync::{mpsc, Notify},
    time::sleep,
};
use tower::{Service, ServiceExt};
use tracing::instrument;

use cuprate_consensus::{BlockChainContext, BlockChainContextRequest, BlockChainContextResponse};
use cuprate_p2p::{
    block_downloader::{BlockBatch, BlockDownloaderConfig, ChainSvcRequest, ChainSvcResponse},
    NetworkInterface,
};
use cuprate_p2p_core::ClearNet;

/// An error returned from the [`syncer`].
#[derive(Debug, thiserror::Error)]
pub enum SyncerError {
    #[error("Incoming block channel closed.")]
    IncomingBlockChannelClosed,
    #[error("One of our services returned an error: {0}.")]
    ServiceError(#[from] tower::BoxError),
}

#[instrument(level = "debug", skip_all)]
pub async fn syncer<C, CN>(
    mut context_svc: C,
    our_chain: CN,
    clearnet_interface: NetworkInterface<ClearNet>,
    incoming_block_batch_tx: mpsc::Sender<BlockBatch>,
    stop_current_block_downloader: Notify,
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

    let BlockChainContextResponse::Context(mut blockchain_ctx) = context_svc
        .ready()
        .await?
        .call(BlockChainContextRequest::GetContext)
        .await?
    else {
        panic!("Blockchain context service returned wrong response!");
    };

    let mut peer_sync_watch = clearnet_interface.top_sync_stream();

    tracing::debug!("Waiting for new sync info in top sync channel");

    while let Some(top_sync_info) = peer_sync_watch.next().await {
        tracing::info!(
            "New sync info seen, top height: {}, top block hash: {}",
            top_sync_info.chain_height,
            hex::encode(top_sync_info.top_hash)
        );

        // The new info could be from a peer giving us a block, so wait a couple seconds to allow the block to
        // be added to our blockchain.
        sleep(Duration::from_secs(2)).await;

        check_update_blockchain_context(&mut context_svc, &mut blockchain_ctx).await?;
        let raw_blockchain_context = blockchain_ctx.unchecked_blockchain_context();

        if top_sync_info.cumulative_difficulty <= raw_blockchain_context.cumulative_difficulty {
            tracing::debug!("New peer sync info is not ahead, nothing to do.");
            continue;
        }

        tracing::debug!(
            "We are behind peers claimed cumulative difficulty, starting block downloader"
        );
        let mut block_batch_stream =
            clearnet_interface.block_downloader(our_chain.clone(), block_downloader_config);

        loop {
            tokio::select! {
                _ = stop_current_block_downloader.notified() => {
                    tracing::info!("Stopping block downloader");
                    break;
                }
                Some(batch) = block_batch_stream.next() => {
                    tracing::debug!("Got batch, len: {}", batch.blocks.len());
                    if incoming_block_batch_tx.send(batch).await.is_err() {
                        return Err(SyncerError::IncomingBlockChannelClosed);
                    }
                }
            }
        }
    }

    Ok(())
}

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
        .oneshot(BlockChainContextRequest::GetContext)
        .await?
    else {
        panic!("Blockchain context service returned wrong response!");
    };

    *old_context = ctx;

    Ok(())
}

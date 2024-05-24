//! # Block Downloader
//!

use std::collections::VecDeque;
use std::sync::Arc;

use monero_serai::{block::Block, transaction::Transaction};
use rand::prelude::*;
use tokio::task::JoinSet;
use tower::{Service, ServiceExt};

use async_buffer::{BufferAppender, BufferStream};
use monero_p2p::{
    handles::ConnectionHandle,
    services::{PeerSyncRequest, PeerSyncResponse},
    NetworkZone, PeerRequest, PeerResponse, PeerSyncSvc,
};
use monero_wire::protocol::ChainRequest;

use crate::client_pool::ClientPool;
use crate::constants::INITIAL_CHAIN_REQUESTS_TO_SEND;

/// A downloaded batch of blocks.
pub struct BlockBatch {
    /// The blocks.
    blocks: (Block, Vec<Transaction>),
    /// The peer that gave us this block.
    peer_handle: ConnectionHandle,
}

/// The block downloader config.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq)]
pub struct BlockDownloaderConfig {
    /// The size of the buffer between the block downloader and the place which
    /// is consuming the downloaded blocks.
    buffer_size: usize,
    /// The size of the in progress queue at which we stop requesting more blocks.
    in_progress_queue_size: usize,
}

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq, thiserror::Error)]
pub enum BlockDownloadError {
    #[error("Failed to find a more advanced chain to follow")]
    FailedToFindAChainToFollow,
    #[error("Service error: {0}")]
    ServiceError(#[from] tower::BoxError),
}

/// The request type for the chain service.
pub enum ChainSvcRequest {
    /// A request for the current chain history.
    CompactHistory,
    /// A request to find the first unknown
    FindFirstUnknown(Vec<[u8; 32]>),
}

/// The response type for the chain service.
pub enum ChainSvcResponse {
    /// The response for [`ChainSvcRequest::CompactHistory`]
    CompactHistory {
        block_ids: Vec<[u8; 32]>,
        cumulative_difficulty: u128,
    },
    /// The response for [`ChainSvcRequest::FindFirstUnknown`], contains the index of the first unknown
    /// block.
    FindFirstUnknown(usize),
}

/// # Block Downloader
///
/// This function starts the block downloader and returns a [`BufferStream`] that will produce
/// a sequential stream of blocks.
///
/// The block downloader will pick the longest chain and will follow it for as long as possible,
/// the blocks given from the [`BufferStream`] will be in order.
///
/// The block downloader may fail before the whole chain is downloaded. If this is the case you are
/// free to call this function again, so it can start the search again.
pub fn download_blocks<N: NetworkZone, S>(
    client_pool: Arc<ClientPool<N>>,
    peer_sync_svc: S,
    config: BlockDownloaderConfig,
) -> BufferStream<BlockBatch> {
    let (buffer_appender, buffer_stream) = async_buffer::new_buffer(config.buffer_size);

    tokio::spawn(block_downloader(
        client_pool,
        peer_sync_svc,
        config,
        buffer_appender,
    ));

    buffer_stream
}

async fn block_downloader<N: NetworkZone, S>(
    client_pool: Arc<ClientPool<N>>,
    peer_sync_svc: S,
    config: BlockDownloaderConfig,
    buffer_appender: BufferAppender<BlockBatch>,
) -> Result<(), tower::BoxError> {
    todo!()
}

struct BestChainFound {
    common_ancestor: [u8; 32],
    next_hashes: VecDeque<[u8; 32]>,
    from_peer: ConnectionHandle,
}

async fn initial_chain_search<N: NetworkZone, S, C>(
    client_pool: &ClientPool<N>,
    mut peer_sync_svc: S,
    mut our_chain_svc: C,
) -> Result<BestChainFound, BlockDownloadError>
where
    S: PeerSyncSvc<N>,
    C: Service<ChainSvcRequest, Response = ChainSvcResponse> + Send + 'static,
    C::Future: Send + 'static,
{
    let ChainSvcResponse::CompactHistory {
        block_ids,
        cumulative_difficulty,
    } = our_chain_svc
        .ready()
        .await?
        .call(ChainSvcRequest::CompactHistory)
        .await?
    else {
        panic!("chain service sent wrong response.");
    };

    let PeerSyncResponse::PeersToSyncFrom(mut peers) = peer_sync_svc
        .ready()
        .await?
        .call(PeerSyncRequest::PeersToSyncFrom {
            block_needed: None,
            current_cumulative_difficulty: cumulative_difficulty,
        })
        .await?
    else {
        panic!("peer sync service sent wrong response.");
    };

    peers.shuffle(&mut thread_rng());

    let mut peers = client_pool.borrow_clients(&peers);

    let mut futs = JoinSet::new();

    let req = PeerRequest::GetChain(ChainRequest {
        block_ids: block_ids.into(),
        prune: false,
    });

    while futs.len() < INITIAL_CHAIN_REQUESTS_TO_SEND {
        let Some(mut next_peer) = peers.next() else {
            break;
        };
        let cloned_req = req.clone();
        futs.spawn(async move {
            let PeerResponse::GetChain(chain_res) =
                next_peer.ready().await?.call(cloned_req).await?
            else {
                panic!("connection task returned wrong response!");
            };

            Ok((chain_res, next_peer.info.handle.clone()))
        });
    }

    let mut res = None;

    while let Some(task_res) = futs.join_next().await {
        let Ok(task_res) = task_res.unwrap() else {
            continue;
        };

        match &mut res {
            Some(res) => {
                if res.0.cumulative_difficulty() < task_res.0.cumulative_difficulty() {
                    let _ = std::mem::replace(res, task_res);
                }
            }
            None => {
                let _ = std::mem::replace(&mut res, Some(task_res));
            }
        }
    }

    let Some((chain_res, peer_handle)) = res else {
        return Err(BlockDownloadError::FailedToFindAChainToFollow);
    };

    let hashes: Vec<[u8; 32]> = chain_res.m_block_ids.into();

    let ChainSvcResponse::FindFirstUnknown(first_unknown) = our_chain_svc
        .ready()
        .await?
        .call(ChainSvcRequest::FindFirstUnknown(hashes.clone()))
        .await?
    else {
        panic!("chain service sent wrong response.");
    };
    
    todo!()
    
}

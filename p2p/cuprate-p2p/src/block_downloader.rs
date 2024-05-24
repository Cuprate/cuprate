//! # Block Downloader
//!

mod chain_tracker;

use std::collections::{BTreeMap, BinaryHeap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use monero_serai::{block::Block, transaction::Transaction};
use rand::prelude::*;
use tokio::task::JoinSet;
use tokio::time::{interval, MissedTickBehavior};
use tower::{Service, ServiceExt};

use crate::block_downloader::chain_tracker::{ChainEntry, ChainTracker};
use async_buffer::{BufferAppender, BufferStream};
use fixed_bytes::ByteArrayVec;
use monero_p2p::client::InternalPeerID;
use monero_p2p::{
    handles::ConnectionHandle,
    services::{PeerSyncRequest, PeerSyncResponse},
    NetworkZone, PeerRequest, PeerResponse, PeerSyncSvc,
};
use monero_wire::protocol::{ChainRequest, ChainResponse};

use crate::client_pool::{ClientPool, ClientPoolDropGuard};
use crate::constants::{INITIAL_CHAIN_REQUESTS_TO_SEND, MEDIUM_BAN};

/// A downloaded batch of blocks.
pub struct BlockBatch {
    /// The blocks.
    blocks: Vec<(Block, Vec<Transaction>)>,
    /// The size of this batch in bytes.
    size: usize,
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
    /// The [`Duration`] between checking the client pool for free peers.
    check_client_pool_interval: Duration,
    /// The target size of a single batch of blocks (in bytes).
    target_batch_size: usize,
    /// The initial amount of blocks to request (in number of blocks)
    initial_batch_size: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum BlockDownloadError {
    #[error("Failed to find a more advanced chain to follow")]
    FailedToFindAChainToFollow,
    #[error("The peer did not send any overlapping blocks, unknown start height.")]
    PeerSentNoOverlappingBlocks,
    #[error("Service error: {0}")]
    ServiceError(#[from] tower::BoxError),
}

/// The request type for the chain service.
pub enum ChainSvcRequest {
    /// A request for the current chain history.
    CompactHistory,
    /// A request to find the first unknown
    FindFirstUnknown(Vec<[u8; 32]>),

    CumulativeDifficulty,
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

    CumulativeDifficulty(u128),
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

    /*
    tokio::spawn(block_downloader(
        client_pool,
        peer_sync_svc,
        config,
        buffer_appender,
    ));

     */

    buffer_stream
}

struct BlockDownloader<N: NetworkZone, S, C> {
    client_pool: Arc<ClientPool<N>>,

    peer_sync_svc: S,
    our_chain_svc: C,

    block_download_tasks: JoinSet<()>,
    chain_entry_task: JoinSet<()>,

    buffer_appender: BufferAppender<BlockBatch>,

    config: BlockDownloaderConfig,
}

async fn block_downloader<N: NetworkZone, S, C>(
    client_pool: Arc<ClientPool<N>>,
    mut peer_sync_svc: S,
    mut our_chain_svc: C,
    config: BlockDownloaderConfig,
    buffer_appender: BufferAppender<BlockBatch>,
) -> Result<(), BlockDownloadError>
where
    S: PeerSyncSvc<N> + Clone,
    C: Service<ChainSvcRequest, Response = ChainSvcResponse, Error = tower::BoxError>
        + Send
        + 'static,
    C::Future: Send + 'static,
{
    let mut best_chain_found =
        initial_chain_search(&client_pool, peer_sync_svc.clone(), &mut our_chain_svc).await?;

    let tasks = JoinSet::new();

    let mut ready_queue = BinaryHeap::new();
    let mut inflight_queue = BTreeMap::new();

    let mut next_request_id = 0;

    // The request ID for which we updated `amount_of_blocks_to_request`
    // `amount_of_blocks_to_request` will update for every new batch of blocks that come in.
    let mut amount_of_blocks_to_request_updated_at = next_request_id;

    // The amount of blocks to request in 1 batch, will dynamically update based on block size.
    let mut amount_of_blocks_to_request = config.initial_batch_size;

    let mut check_client_pool_interval = interval(config.check_client_pool_interval);
    check_client_pool_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = check_client_pool_interval.tick() => {
                todo!()
            }



        }
    }
}

async fn handle_free_peer<N: NetworkZone>(
    peer: ClientPoolDropGuard<N>,
    chain_tracker: &mut ChainTracker<N>,
    next_batch_size: usize,
) {
    if chain_tracker.block_requests_queued(next_batch_size) < 15
        && chain_tracker.should_ask_for_next_chain_entry(&peer.info.pruning_seed)
    {}
}

async fn initial_chain_search<N: NetworkZone, S, C>(
    client_pool: &Arc<ClientPool<N>>,
    mut peer_sync_svc: S,
    mut our_chain_svc: C,
) -> Result<ChainTracker<N>, BlockDownloadError>
where
    S: PeerSyncSvc<N>,
    C: Service<ChainSvcRequest, Response = ChainSvcResponse, Error = tower::BoxError>,
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

    let our_genesis = *block_ids.last().expect("Blockchain had no genesis block.");

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

            Ok((chain_res, next_peer.info.id, next_peer.info.handle.clone()))
        });
    }

    let mut res: Option<(ChainResponse, InternalPeerID<_>, ConnectionHandle)> = None;

    while let Some(task_res) = futs.join_next().await {
        let Ok(task_res): Result<
            (ChainResponse, InternalPeerID<_>, ConnectionHandle),
            tower::BoxError,
        > = task_res.unwrap() else {
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

    let Some((chain_res, peer_id, peer_handle)) = res else {
        return Err(BlockDownloadError::FailedToFindAChainToFollow);
    };

    let hashes: Vec<[u8; 32]> = (&chain_res.m_block_ids).into();
    let start_height = chain_res.start_height;
    // drop this to deallocate the [`Bytes`].
    drop(chain_res);

    let ChainSvcResponse::FindFirstUnknown(first_unknown) = our_chain_svc
        .ready()
        .await?
        .call(ChainSvcRequest::FindFirstUnknown(hashes.clone()))
        .await?
    else {
        panic!("chain service sent wrong response.");
    };

    if first_unknown == 0 {
        peer_handle.ban_peer(MEDIUM_BAN);
        return Err(BlockDownloadError::PeerSentNoOverlappingBlocks);
    }

    let first_entry = ChainEntry {
        ids: hashes[first_unknown..].to_vec(),
        peer: peer_id,
        handle: peer_handle,
    };

    let tracker = ChainTracker::new(first_entry, start_height, our_genesis);

    Ok(tracker)
}

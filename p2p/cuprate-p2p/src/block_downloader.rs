use crate::peer_set::{ClientPool, ClientPoolGuard};
use async_buffer::{new_buffer, BufferAppender, BufferStream};
use futures::StreamExt;
use monero_p2p::{NetworkZone, PeerSyncSvc};
use monero_serai::block::Block;
use monero_serai::transaction::Transaction;
use std::cmp::min;
use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::{interval, Instant, Interval, MissedTickBehavior};
use tokio_stream::wrappers::IntervalStream;
use tokio_util::sync::CancellationToken;
use tower::{Service, ServiceExt};

use crate::constants::{
    CHAIN_REQUESTS_TO_SEND, INCOMING_BLOCKS_CACHE_SIZE, NUMBER_OF_BLOCKS_TO_REQUEST,
};
use fixed_bytes::ByteArrayVec;
use monero_p2p::client::InternalPeerID;
use monero_p2p::handles::ConnectionHandle;
use monero_p2p::services::{PeerSyncRequest, PeerSyncResponse};

mod block_download_task;
mod chain_entry_task;
mod chains;

use crate::block_downloader::chains::ChainEntry;
use block_download_task::{DownloadBlocksError, DownloadedBlocks, GetBlocksOk};
use chain_entry_task::{ChainEntryTaskErr, ChainEntryTaskOk};
use chains::ChainTracker;
use monero_pruning::CRYPTONOTE_MAX_BLOCK_HEIGHT;
use monero_wire::protocol::ChainResponse;

pub async fn download_blocks<N: NetworkZone, PSync, BC>(
    peer_sync_svc: PSync,
    client_pool: Arc<ClientPool<N>>,
    our_chain: BC,
) -> BufferStream<IncomingBlocks>
where
    PSync: PeerSyncSvc<N> + Send + 'static + Clone,
    BC: Blockchain + Send + 'static + Clone,
{
    let (buffer_tx, buffer_rx) = new_buffer(INCOMING_BLOCKS_CACHE_SIZE);

    let downloader = BlockDownloader::new(
        peer_sync_svc.clone(),
        our_chain.clone(),
        client_pool,
        buffer_tx,
    )
    .await;

    tokio::spawn(downloader.run());

    buffer_rx
}

pub struct IncomingBlocks {
    pub blocks: Vec<(Block, Vec<Transaction>)>,
    pub peer_handle: ConnectionHandle,
}

struct InflightBlockRequest {
    ids: ByteArrayVec<32>,
    expected_start_height: u64,

    failed: bool,

    ready: Option<DownloadedBlocks>,
    cancel: CancellationToken,
}

struct PendingChainRequest {
    ids: Vec<[u8; 32]>,
    requests_sent: usize,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Where {
    NotFound,
    MainChain(u64),
    AltChain(u64),
    Invalid,
}

pub trait Blockchain {
    fn cumulative_difficulty(&mut self) -> impl Future<Output = u128> + Send;

    fn have_block(&mut self, block: [u8; 32]) -> impl Future<Output = Where> + Send;

    fn history(&mut self) -> impl Future<Output = Vec<[u8; 32]>> + Send;

    fn chain_height(&mut self) -> impl Future<Output = u64> + Send;
}

pub struct BlockDownloader<N: NetworkZone, PSync, BC> {
    peer_sync_svc: PSync,
    our_chain: BC,
    client_pool: Arc<ClientPool<N>>,

    block_download_tasks: JoinSet<Result<GetBlocksOk<N>, DownloadBlocksError>>,
    chain_entries_task: JoinSet<Result<ChainEntryTaskOk<N>, ChainEntryTaskErr>>,
    check_client_pool_interval: IntervalStream,

    chain_tracker: ChainTracker<N>,
    top_found: bool,

    pending_chain_request: Option<PendingChainRequest>,
    in_flight_requests: BTreeMap<u64, InflightBlockRequest>,
    buffer: BufferAppender<IncomingBlocks>,
}

impl<N: NetworkZone, PSync, BC> BlockDownloader<N, PSync, BC>
where
    PSync: PeerSyncSvc<N> + Send + 'static + Clone,
    BC: Blockchain,
{
    async fn new(
        mut peer_sync_svc: PSync,
        mut our_chain: BC,
        client_pool: Arc<ClientPool<N>>,
        buffer: BufferAppender<IncomingBlocks>,
    ) -> Self {
        let chain_height = our_chain.chain_height().await;
        let current_cumulative_difficulty = our_chain.cumulative_difficulty().await;
        let history = our_chain.history().await;
        let genesis = *history.last().unwrap();

        let PeerSyncResponse::PeersToSyncFrom(peers) = peer_sync_svc
            .ready()
            .await
            .expect("Internal service error in the peer sync service.")
            .call(PeerSyncRequest::PeersToSyncFrom {
                current_cumulative_difficulty,
                block_needed: Some(chain_height),
            })
            .await
            .expect("Internal service error in the peer sync service.")
        else {
            panic!("Peer sync service sent incorrect response!");
        };

        let peers = client_pool.borrow_clients(&peers);
        let mut tasks = JoinSet::new();

        for peer in peers.into_iter().take(CHAIN_REQUESTS_TO_SEND) {
            tasks.spawn(chain_entry_task::get_next_chain_entry(
                peer,
                history.clone(),
            ));
        }

        let (start_height, next_entry) = loop {
            let Some(Ok(chain_entry)) = tasks.join_next().await else {
                todo!();
            };
            let Ok(mut entry) = chain_entry else {
                continue;
            };

            let mut start_height = entry.chain_entry.start_height;
            let mut entry = ChainEntry {
                ids: (&entry.chain_entry.m_block_ids).into(),
                peer: entry.client.info.id,
                handle: entry.client.info.handle.clone(),
            };

            let new = find_new(&mut our_chain, &entry.ids, start_height)
                .await
                .expect("TODO");

            entry.ids.drain(0..new);
            start_height += u64::try_from(new).unwrap();

            break (start_height, entry);
        };

        let mut interval = interval(Duration::from_secs(15));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        Self {
            peer_sync_svc,
            our_chain,
            client_pool,
            block_download_tasks: Default::default(),
            chain_entries_task: Default::default(),
            check_client_pool_interval: IntervalStream::new(interval),
            top_found: false,
            chain_tracker: ChainTracker::new(next_entry, start_height, genesis),

            in_flight_requests: Default::default(),
            buffer,
            pending_chain_request: None,
        }
    }

    fn request_next_chain_entry(&mut self, client: ClientPoolGuard<N>) -> bool {
        if !self
            .chain_tracker
            .should_ask_for_next_chain_entry(&client.info.pruning_seed)
        {
            return false;
        }

        let pending_chain_request = self
            .pending_chain_request
            .get_or_insert(PendingChainRequest {
                ids: self.chain_tracker.get_simple_history(),
                requests_sent: 0,
            });

        if pending_chain_request.requests_sent > CHAIN_REQUESTS_TO_SEND {
            return false;
        }

        pending_chain_request.requests_sent += 1;

        self.chain_entries_task
            .spawn(chain_entry_task::get_next_chain_entry(
                client,
                pending_chain_request.ids.clone(),
            ));

        true
    }

    fn try_request_next_blocks(&mut self, client: ClientPoolGuard<N>) {
        let has_block = |height| {
            client
                .info
                .pruning_seed
                .has_full_block(height, CRYPTONOTE_MAX_BLOCK_HEIGHT)
        };

        for (request_id, request) in &mut self.in_flight_requests {
            if request.failed
                && has_block(request.expected_start_height)
                && has_block(
                    request.expected_start_height + u64::try_from(request.ids.len()).unwrap(),
                )
            {
                request.failed = false;

                self.block_download_tasks
                    .spawn(block_download_task::get_blocks(
                        client,
                        request.ids.clone(),
                        *request_id,
                        request.cancel.clone(),
                    ));

                return;
            }
        }

        let Some(next_entry) = self
            .chain_tracker
            .blocks_to_get(&client.info.pruning_seed, NUMBER_OF_BLOCKS_TO_REQUEST)
        else {
            return;
        };

        let request_id = self
            .in_flight_requests
            .last_key_value()
            .map(|(key, _)| *key)
            .unwrap_or(0)
            + 1;

        let cancellation_token = CancellationToken::new();

        self.block_download_tasks
            .spawn(block_download_task::get_blocks(
                client,
                next_entry.ids.clone().into(),
                request_id,
                cancellation_token.clone(),
            ));

        let request = InflightBlockRequest {
            ids: next_entry.ids.into(),
            expected_start_height: next_entry.start_height,
            ready: None,
            cancel: cancellation_token,
            failed: false,
        };

        self.in_flight_requests.insert(request_id, request);
    }

    fn handle_free_client(&mut self, client: ClientPoolGuard<N>) {
        let queued_block_entries = self
            .chain_tracker
            .block_requests_queued(NUMBER_OF_BLOCKS_TO_REQUEST);

        if queued_block_entries <= 20
            && (self.pending_chain_request.is_none()
                || self
                    .pending_chain_request
                    .as_ref()
                    .is_some_and(|pending| pending.requests_sent < CHAIN_REQUESTS_TO_SEND))
        {
            self.request_next_chain_entry(client);
        } else {
            self.try_request_next_blocks(client);
        }
    }

    async fn check_for_free_peers(&mut self) {
        let current_cumulative_difficulty = self.our_chain.cumulative_difficulty().await;

        let PeerSyncResponse::PeersToSyncFrom(peers) = self
            .peer_sync_svc
            .ready()
            .await
            .expect("Internal service error in the peer sync service.")
            .call(PeerSyncRequest::PeersToSyncFrom {
                current_cumulative_difficulty,
                block_needed: None,
            })
            .await
            .expect("Internal service error in the peer sync service.")
        else {
            panic!("Peer sync service sent incorrect response!");
        };

        if peers.is_empty() {
            return;
        }

        tracing::warn!("checking peer set");
        let peers = self.client_pool.borrow_clients(&peers);

        peers
            .into_iter()
            .for_each(|peer| self.handle_free_client(peer));

        tracing::warn!("checked peer set");
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(block_res) = self.block_download_tasks.join_next() => {
                    match block_res {
                        Ok(Ok(downloaded_blocks)) => {
                            let Some(in_flight_request) = self.in_flight_requests.get_mut(&downloaded_blocks.request_id) else {
                                self.handle_free_client(downloaded_blocks.client);
                                continue;
                            };

                            let _ = in_flight_request.ready.insert(downloaded_blocks.blocks);

                            while self.in_flight_requests.first_key_value().is_some_and(|(_, value)| value.ready.is_some()) {
                                let Some(Some(ready_blocks)) = self.in_flight_requests.pop_first().map(|req| req.1.ready) else {
                                    unreachable!();
                                };

                                if self.buffer.send(IncomingBlocks {blocks: ready_blocks.blocks, peer_handle: downloaded_blocks.client.info.handle.clone()}, ready_blocks.size).await.is_err() {
                                    return;
                                }
                            }

                            self.handle_free_client(downloaded_blocks.client);
                            continue;
                        },
                        Ok(Err(block_download_err)) => {
                            let Some(in_flight_request) = self.in_flight_requests.get_mut(&block_download_err.request_id) else {
                                continue;
                            };

                            tracing::warn!("Error downloading blocks: {:?}", block_download_err.error);

                            in_flight_request.failed = true;
                        },
                        Err(e) if e.is_panic() => {
                            std::panic::resume_unwind(e.into_panic());
                        }
                        Err(_) => continue,
                    }
                },
                Some(Ok(Ok(chain_entry))) = self.chain_entries_task.join_next() => {

                    if self.pending_chain_request.is_none() {
                        self.handle_free_client(chain_entry.client);
                        continue;
                    }

                    match self.chain_tracker.add_entry(chain_entry.chain_entry, chain_entry.client.info.id, chain_entry.client.info.handle.clone()) {
                        Ok(()) =>  {
                            self.pending_chain_request.take();
                        }
                        Err(_) => {
                           tracing::warn!("Error adding next block entry.");

                            if self.chain_entries_task.is_empty() {
                                tracing::warn!("Top found exiting");
                                panic!();
                            }
                        }
                    }

                    self.handle_free_client(chain_entry.client);
                    continue;
                }
                _ = self.check_client_pool_interval.next() => {
                    self.check_for_free_peers().await;
                }
            }
        }
    }
}

/// Does a binary search on the incoming block hashes to find the index of the first hash we
/// don't know about.
///
/// Will error if we encounter a hash of a block that we have marked as invalid.
async fn find_new<BC: Blockchain>(
    blockchain: &mut BC,
    incoming_chain: &[[u8; 32]],
    start_height: u64,
) -> Result<usize, ()>
where
    BC: Blockchain,
{
    let mut size = incoming_chain.len();
    let mut left = 0;
    let mut right = size;

    while left < right {
        let mid = left + size / 2;

        let have_block = blockchain.have_block(incoming_chain[mid]).await;

        match have_block {
            Where::Invalid => return Err(()),
            Where::AltChain(height) | Where::MainChain(height) => {
                if height != start_height + u64::try_from(mid).unwrap() {
                    return Err(());
                }

                left = mid + 1;
            }
            Where::NotFound => {
                right = mid;
            }
        }

        size = right - left;
    }

    Ok(left)
}

/*

use std::pin::Pin;
use std::{cmp::min, collections::VecDeque, fmt::Debug, future::Future, panic};

use futures::{stream::FuturesUnordered, FutureExt, StreamExt};

use monero_serai::{block::Block, transaction::Transaction};
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time::{interval, sleep, timeout, MissedTickBehavior, Sleep};
use tokio_stream::wrappers::IntervalStream;
use tokio_util::sync::{CancellationToken, ReusableBoxFuture};
use tower::{Service, ServiceExt};

use async_buffer::{new_buffer, BufferAppender, BufferStream};
use fixed_bytes::ByteArrayVec;
use monero_p2p::{
    client::InternalPeerID,
    handles::ConnectionHandle,
    services::{PeerSyncRequest, PeerSyncResponse},
    NetworkZone, PeerRequest, PeerResponse, PeerSyncSvc,
};
use monero_wire::protocol::{ChainRequest, GetObjectsRequest};

use crate::{
    constants::{
        BLOCK_REQUEST_TIMEOUT, BLOCK_REQUEST_TIMEOUT_INTERVAL, CHAIN_REQUEST_TIMEOUT,
        CONCURRENT_BLOCKS_REQUESTS, INCOMING_BLOCKS_CACHE_SIZE, MEDIUM_BAN,
        NUMBER_OF_BLOCKS_TO_REQUEST,
    },
    peer_set::{PeerSet, PeerSetRequest, PeerSetResponse},
};

mod block_download_task;
mod chain_entry_task;

use crate::block_downloader::block_download_task::get_blocks;
use crate::block_downloader::chain_entry_task::{get_next_chain_entry, ChainEntry};
use block_download_task::{DownloadBlocksError, DownloadedBlocks, GetBlocksOk};
use chain_entry_task::{ChainEntryTaskErr, ChainEntryTaskOk};

#[derive(Debug)]
pub enum BlockDownloaderError {
    BlockInvalid,
    PeerGaveInvalidInfo,
    PeerDoesNotHaveData,
    InternalSvc(tower::BoxError),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Where {
    NotFound,
    MainChain(u64),
    AltChain(u64),
    Invalid,
}

pub trait Blockchain {
    fn chain_history(
        &mut self,
        from: Option<[u8; 32]>,
    ) -> impl Future<Output = Vec<[u8; 32]>> + Send;

    fn have_block(&mut self, block_id: [u8; 32]) -> impl Future<Output = Where> + Send;

    fn cumulative_difficulty(&mut self) -> impl Future<Output = u128> + Send;

    fn current_height(&mut self) -> impl Future<Output = u64> + Send;

    fn top_hash(&mut self) -> impl Future<Output = [u8; 32]> + Send;
}

pub async fn download_blocks<N: NetworkZone, PSet, PSync, BC>(
    peer_sync_svc: PSync,
    peer_set: PSet,
    our_chain: BC,
) -> BufferStream<Vec<(Block, Vec<Transaction>)>>
where
    PSet: Service<PeerSetRequest<N>, Response = PeerSetResponse<N>, Error = tower::BoxError>
        + Send
        + 'static
        + Clone,
    PSet::Future: Send + 'static,
    PSync: PeerSyncSvc<N> + Send + 'static + Clone,
    BC: Blockchain + Send + 'static + Clone,
{
    let (buffer_tx, buffer_rx) = new_buffer(INCOMING_BLOCKS_CACHE_SIZE);
    let (chain_finder_tx, chain_finder_rx) = mpsc::channel(15);

    let downloader = BlockDownloader2::new(
        peer_sync_svc.clone(),
        peer_set.clone(),
        our_chain.clone(),
        chain_finder_rx,
        buffer_tx,
    )
    .await;

    let chain_finder = chain_finder::ChainFinder::new(
        peer_set.clone(),
        peer_sync_svc.clone(),
        our_chain.clone(),
        chain_finder_tx,
    )
    .await;

    tokio::spawn(chain_finder.run());

    tokio::spawn(downloader.run());

    buffer_rx
}

pub struct InflightRequest<N: NetworkZone> {
    ids: ByteArrayVec<32>,
    request_id: u64,

    timed_out: bool,

    ready: Option<DownloadedBlocks>,
    cancel_token: CancellationToken,

    peer_that_told_us: InternalPeerID<N::Addr>,
    peer_that_told_us_handle: ConnectionHandle,
}

impl<N: NetworkZone> InflightRequest<N> {
    pub fn new(
        ids: ByteArrayVec<32>,
        request_id: u64,
        peer_that_told_us: InternalPeerID<N::Addr>,
        peer_that_told_us_handle: ConnectionHandle,
    ) -> Self {
        InflightRequest {
            ids,
            request_id,
            timed_out: false,
            ready: None,
            cancel_token: CancellationToken::new(),
            peer_that_told_us,
            peer_that_told_us_handle,
        }
    }
}

impl<N: NetworkZone> Drop for InflightRequest<N> {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}

pub struct BlockDownloader2<N: NetworkZone, PSet, PSync, BC> {
    peer_sync_svc: PSync,
    peer_set: PSet,
    our_chain: BC,

    check_peer_set_interval: IntervalStream,

    request_tasks: JoinSet<Result<GetBlocksOk<N>, DownloadBlocksError>>,
    chain_entry_tasks: JoinSet<Result<ChainEntryTaskOk<N>, ChainEntryTaskErr>>,

    genesis_hash: [u8; 32],
    last_hash: Option<[u8; 32]>,
    chain_entries: VecDeque<ChainEntry<N>>,

    in_flight_requests: VecDeque<InflightRequest<N>>,
    buffer: BufferAppender<Vec<(Block, Vec<Transaction>)>>,
}

impl<N: NetworkZone, PSet, PSync, BC> BlockDownloader2<N, PSet, PSync, BC>
where
    //    PSet: Service<PeerSetRequest<N>, Response = PeerSetResponse<N>, Error = tower::BoxError>,
    PSet::Future: Send + 'static,
    PSync: PeerSyncSvc<N>,
    BC: Blockchain,
{
    fn chain_entries_in_queue(&self, block_batch_size: usize) -> usize {
        self.chain_entries
            .iter()
            .map(|entry| entry.ids.len().div_ceil(block_batch_size))
            .sum()
    }

    async fn history(&mut self) -> Vec<[u8; 32]> {
        if let Some(last_hash) = self.last_hash {
            return vec![last_hash, self.genesis_hash];
        }

        self.our_chain.chain_history(None).await
    }

    /*
    async fn new(
        peer_sync_svc: PSync,
        peer_set: PSet,
        mut our_chain: BC,
        chain_finder: mpsc::Receiver<ChainEntry<N>>,
        buffer: BufferAppender<Vec<(Block, Vec<Transaction>)>>,
    ) -> Self {
        let mut timeout = interval(BLOCK_REQUEST_TIMEOUT_INTERVAL);

        timeout.set_missed_tick_behavior(MissedTickBehavior::Delay);

        Self {
            peer_sync_svc,
            peer_set,
            our_chain,
            in_flight_requests: Default::default(),
            request_futs: Default::default(),
            timeout: IntervalStream::new(timeout),
            buffer,
            tip_found: false,
            chain_finder,
        }
    }

    /// Uses the peer sync service and the peer set to find a peer to send a request for blocks to.
    ///
    /// The caller must keep `in_flight_requests` up to date.
    async fn request_blocks(
        &mut self,
        ids: ByteArrayVec<32>,
        request_id: u64,
        cancel_token: CancellationToken,
        height: Option<u64>,
        peer: Option<InternalPeerID<N::Addr>>,
    ) -> Result<(), BlockDownloaderError> {
        let req = PeerRequest::GetObjects(GetObjectsRequest {
            blocks: ids.clone(),
            pruned: false,
        });

        // This may be a little less than the cumulative difficulty of the last retried block but this will
        // only cause us to send the request to a peer who also may be syncing around our height which would be
        // rare.
        let current_cumulative_difficulty = self.our_chain.cumulative_difficulty().await;

        let req_fut = match peer {
            None => {
                let PeerSyncResponse::PeersToSyncFrom(peers_to_sync_from) = self
                    .peer_sync_svc
                    .ready()
                    .await
                    .map_err(BlockDownloaderError::InternalSvc)?
                    .call(PeerSyncRequest::PeersToSyncFrom {
                        current_cumulative_difficulty,
                        block_needed: height,
                    })
                    .await
                    .map_err(BlockDownloaderError::InternalSvc)?
                else {
                    panic!("Peer sync service snt incorrect response.");
                };

                tokio::spawn(timeout(
                    BLOCK_REQUEST_TIMEOUT,
                    self.peer_set
                        .ready()
                        .await
                        .map_err(BlockDownloaderError::InternalSvc)?
                        .call(PeerSetRequest::LoadBalancedPeerSubSetRequest {
                            peers: peers_to_sync_from,
                            req,
                        }),
                ))
                .map(|res| res??)
                .boxed()
            }
            Some(peer) => tokio::spawn(timeout(
                BLOCK_REQUEST_TIMEOUT,
                self.peer_set
                    .ready()
                    .await
                    .map_err(BlockDownloaderError::InternalSvc)?
                    .call(PeerSetRequest::RequestToSpecificPeer { peer, req }),
            ))
            .map(|res| res??)
            .boxed(),
        };

        self.request_futs.push(BlockDownloadFuture::new(
            ids,
            req_fut,
            peer.is_some(),
            request_id,
            cancel_token,
        ));

        Ok(())
    }

    async fn request_next_batch(&mut self) -> Result<(), BlockDownloaderError> {
        let Some(chain_entry) = self.chain_finder.recv().await else {
            self.tip_found = true;
            return Ok(());
        };

        let ids: ByteArrayVec<32> = chain_entry.ids.into();

        let request_id = self
            .in_flight_requests
            .back()
            .map_or(0, |flight| flight.request_id)
            + 1;

        let inflight_request = InflightRequest::new(
            ids.clone(),
            request_id,
            chain_entry.peer_that_told_us,
            chain_entry.peer_that_told_us_handle.clone(),
        );

        self.request_blocks(
            ids,
            request_id,
            inflight_request.cancel_token.clone(),
            Some(chain_entry.start_height),
            None,
        )
        .await?;

        self.in_flight_requests.push_back(inflight_request);
        Ok(())
    }

     */

    async fn run(mut self) -> Result<(), BlockDownloaderError> {
        todo!()
        /*
        loop {
            tokio::select! {
                _ = self.check_peer_set_interval.next() => {
                    todo!()
                }

                Some(res) = self.request_tasks.join_next() => match res {
                    Err(e) => {
                        if e.is_panic() {
                            panic::resume_unwind(e.into_panic());
                        }
                    }
                    Ok(Ok(get_blocks_res)) => {
                        // find the inflight request holder
                        let Some(index) = self
                            .in_flight_requests
                            .iter()
                            .position(|inflight| inflight.request_id == get_blocks_res.request_id)
                        else {
                            // If we arnt waiting for this id then just continue we probably already got it.
                            continue;
                        };

                        // Set the data.
                        self.in_flight_requests[index]
                            .ready
                            .replace(get_blocks_res.blocks);

                        // for every request before this one set it as timed out.
                        for i in 0..index {
                            if self.in_flight_requests[i].ready.is_none() {
                                self.in_flight_requests[i].timed_out = true;
                            }
                        }

                        // Remove all the ready blocks at the start of the queue.
                        while self
                            .in_flight_requests
                            .front()
                            .is_some_and(|inflight| inflight.ready.is_some())
                        {
                            let Some(Some(downloaded_blocks)) = self
                                .in_flight_requests
                                .pop_front()
                                .map(|mut complete| complete.ready.take())
                            else {
                                // We just checked this was some.
                                unreachable!()
                            };

                            // add the blocks to the buffer.
                            if self
                                .buffer
                                .send(downloaded_blocks.blocks, downloaded_blocks.size)
                                .await
                                .is_err()
                            {
                                tracing::info!("Block buffer disconnected, stopping block download.");
                                return Ok(());
                            }
                        }

                        if self.chain_entries_in_queue(NUMBER_OF_BLOCKS_TO_REQUEST) < 15 && self.chain_entry_tasks.is_empty() {
                            self.chain_entry_tasks.spawn(get_next_chain_entry(get_blocks_res.client, self.history().await));
                        } else {

                        }




                    }
                    Err((e, request_id)) => {
                        // find the inflight request holder
                        let Some(index) = self
                            .in_flight_requests
                            .iter()
                            .position(|inflight| inflight.request_id == request_id)
                        else {
                            // If we arnt waiting for this id then just continue we probably already got it.
                            continue;
                        };

                        if self.in_flight_requests[index]
                            .peer_that_told_us_handle
                            .is_closed()
                        {
                            tracing::info!(
                                "Peer {} told us about blocks we can't find and then disconnected.",
                                self.in_flight_requests[index].peer_that_told_us
                            );
                            return Ok(());
                        }

                        tracing::warn!("Error {e:?} getting blocks, getting from peer that told us about them.");

                        self.request_blocks(
                            self.in_flight_requests[index].ids.clone(),
                            request_id,
                            self.in_flight_requests[index].cancel_token.clone(),
                            None,
                            Some(self.in_flight_requests[index].peer_that_told_us)
                        )
                        .await?;
                    }
                }
            }
        }

         */
    }
}


 */

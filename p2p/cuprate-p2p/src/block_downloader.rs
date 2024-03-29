use std::pin::Pin;
use std::{cmp::min, collections::VecDeque, fmt::Debug, future::Future};

use futures::{stream::FuturesUnordered, FutureExt, StreamExt};

use monero_serai::{block::Block, transaction::Transaction};
use tokio::time::{interval, sleep, timeout, MissedTickBehavior, Sleep};
use tokio_stream::wrappers::IntervalStream;
use tokio_util::sync::CancellationToken;
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

mod block_batch_fut;
mod chain_entry;

use block_batch_fut::{BlockDownloadFuture, DownloadedBlocks};
use chain_entry::get_next_chain_entry;

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

pub async fn download_blocks<N: NetworkZone, PSync, BC>(
    mut peer_sync_svc: PSync,
    mut peer_set: PeerSet<N>,
    mut our_chain: BC,
) -> BufferStream<Vec<(Block, Vec<Transaction>)>>
where
    PSync: PeerSyncSvc<N> + Send + 'static,
    BC: Blockchain + Send + 'static,
{
    let (buffer_tx, buffer_rx) = new_buffer(INCOMING_BLOCKS_CACHE_SIZE);

    let downloader = BlockDownloader2::new(peer_sync_svc, peer_set, our_chain, buffer_tx).await;

    tokio::spawn(downloader.run());

    buffer_rx
}

pub struct NextChainEntry<N: NetworkZone> {
    next_ids: Vec<[u8; 32]>,

    peer: InternalPeerID<N::Addr>,
    handle: ConnectionHandle,
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

pub struct BlockDownloader2<N: NetworkZone, PSync, BC> {
    peer_sync_svc: PSync,
    peer_set: PeerSet<N>,
    our_chain: BC,

    in_flight_requests: VecDeque<InflightRequest<N>>,
    request_futs: FuturesUnordered<BlockDownloadFuture<N>>,

    chain_entry: Option<NextChainEntry<N>>,
    tip_found: bool,
    last_hash: [u8; 32],

    timeout: IntervalStream,

    buffer: BufferAppender<Vec<(Block, Vec<Transaction>)>>,
}

impl<N: NetworkZone, PSync, BC> BlockDownloader2<N, PSync, BC>
where
    PSync: PeerSyncSvc<N>,
    BC: Blockchain,
{
    async fn new(
        peer_sync_svc: PSync,
        peer_set: PeerSet<N>,
        mut our_chain: BC,
        buffer: BufferAppender<Vec<(Block, Vec<Transaction>)>>,
    ) -> Self {
        let mut timeout = interval(BLOCK_REQUEST_TIMEOUT_INTERVAL);

        timeout.set_missed_tick_behavior(MissedTickBehavior::Delay);

        Self {
            last_hash: our_chain.top_hash().await,
            peer_sync_svc,
            peer_set,
            our_chain,
            in_flight_requests: Default::default(),
            request_futs: Default::default(),
            chain_entry: None,
            tip_found: false,
            timeout: IntervalStream::new(timeout),
            buffer,
        }
    }

    /// Uses the peer sync service and the peer set to find a peer to send a request for blocks to.
    ///
    /// The caller must keep `in_flight_requests` up to date.
    async fn request_blocks(
        &mut self,
        ids: ByteArrayVec<32>,
        request_id: u64,
        drop_peer_if_not_found: bool,
        cancel_token: CancellationToken,
    ) -> Result<(), BlockDownloaderError> {
        let req = PeerRequest::GetObjects(GetObjectsRequest {
            blocks: ids.clone(),
            pruned: false,
        });

        // This may be a little less than the cumulative difficulty of the last retried block but this will
        // only cause us to send the request to a peer who also may be syncing around our height which would be
        // rare.
        let current_cumulative_difficulty = self.our_chain.cumulative_difficulty().await;

        let PeerSyncResponse::PeersToSyncFrom(peers_to_sync_from) = self
            .peer_sync_svc
            .ready()
            .await
            .map_err(BlockDownloaderError::InternalSvc)?
            .call(PeerSyncRequest::PeersToSyncFrom(
                current_cumulative_difficulty,
            ))
            .await
            .map_err(BlockDownloaderError::InternalSvc)?
        else {
            panic!("Peer sync service snt incorrect response.");
        };

        let req_fut = timeout(
            BLOCK_REQUEST_TIMEOUT,
            self.peer_set
                .ready()
                .await
                .map_err(BlockDownloaderError::InternalSvc)?
                .call(PeerSetRequest::LoadBalancedPeerSubSetRequest {
                    peers: peers_to_sync_from,
                    req,
                }),
        )
        .map(|res| Ok(res??))
        .boxed();

        self.request_futs.push(BlockDownloadFuture::new(
            ids,
            req_fut,
            drop_peer_if_not_found,
            request_id,
            cancel_token,
        ));

        Ok(())
    }

    async fn request_next_batch(&mut self) -> Result<(), BlockDownloaderError> {
        if self.tip_found {
            return Ok(());
        }

        // if chain_entry is none or the amount of blocks in the entry is 0.
        if !self
            .chain_entry
            .as_ref()
            .is_some_and(|entry| !entry.next_ids.is_empty())
        {
            let Some(next_chain_entry) = get_next_chain_entry(
                &mut self.peer_sync_svc,
                &mut self.peer_set,
                &mut self.our_chain,
                Some(self.last_hash),
            )
            .await?
            else {
                self.tip_found = true;
                return Ok(());
            };

            self.last_hash = *next_chain_entry.next_ids.last().unwrap();
            self.chain_entry = Some(next_chain_entry);
        }

        let chain_entry = self.chain_entry.as_mut().unwrap();

        let ids: ByteArrayVec<32> = chain_entry
            .next_ids
            .drain(0..min(NUMBER_OF_BLOCKS_TO_REQUEST, chain_entry.next_ids.len()))
            .collect::<Vec<_>>()
            .into();

        let request_id = self
            .in_flight_requests
            .back()
            .map_or(0, |flight| flight.request_id)
            + 1;

        let inflight_request = InflightRequest::new(
            ids.clone(),
            request_id,
            chain_entry.peer,
            chain_entry.handle.clone(),
        );

        self.request_blocks(
            ids,
            request_id,
            false,
            inflight_request.cancel_token.clone(),
        )
        .await?;
        self.in_flight_requests.push_back(inflight_request);
        Ok(())
    }

    async fn run(mut self) -> Result<(), BlockDownloaderError> {
        loop {
            tokio::select! {
                _ = self.timeout.next() => {
                    let mut requests_to_make = Vec::new();

                    for timed_out_req in self.in_flight_requests.iter_mut().filter(|req| req.timed_out && req.ready.is_none()) {
                        requests_to_make.push((timed_out_req.ids.clone(), timed_out_req.request_id, timed_out_req.cancel_token.clone()));
                        timed_out_req.timed_out = false;
                    }

                    for (ids, request_id, token) in requests_to_make {

                        tracing::warn!("request {} timed out sending another.", request_id);
                        self.request_blocks(ids, request_id, false, token).await?
                    }
                }

                res = self.request_futs.next() => match res {
                    None => {
                        if self.tip_found {
                            return Ok(());
                        }

                        for _ in 0..CONCURRENT_BLOCKS_REQUESTS {
                            self.request_next_batch().await?;
                        }
                    }
                    Some(Ok(downloaded_block)) => {
                        // find the inflight request holder
                        let Some(index) = self
                            .in_flight_requests
                            .iter()
                            .position(|inflight| inflight.request_id == downloaded_block.request_id)
                        else {
                            // If we arnt waiting for this id then just continue we probably already got it.
                            continue;
                        };

                        // Set the data.
                        self.in_flight_requests[index]
                            .ready
                            .replace(downloaded_block);

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

                            // request some more blocks to maintain the number of inflight requests
                            self.request_next_batch().await?;
                        }
                    }
                    Some(Err((_, request_id))) => {
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

                        self.request_blocks(
                            self.in_flight_requests[index].ids.clone(),
                            request_id,
                            true,
                            self.in_flight_requests[index].cancel_token.clone(),
                        )
                        .await?;
                    }
                }
            }
        }
    }
}

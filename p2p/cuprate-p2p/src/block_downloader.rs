//! # Block Downloader
//!
//! This module contains the block downloader, which finds a chain to download from our connected peers
//! and downloads it.
//!
//! The block downloader is started by [`download_blocks`].
use std::{
    cmp::{max, min, Ordering, Reverse},
    collections::{BTreeMap, BinaryHeap, HashSet},
    mem,
    sync::Arc,
    time::Duration,
};

use monero_serai::{block::Block, transaction::Transaction};
use rand::prelude::*;
use rayon::prelude::*;
use tokio::{
    task::JoinSet,
    time::{interval, timeout, MissedTickBehavior},
};
use tower::{Service, ServiceExt};
use tracing::{instrument, Instrument, Span};

use async_buffer::{BufferAppender, BufferStream};
use cuprate_helper::asynch::rayon_spawn_async;
use fixed_bytes::ByteArrayVec;
use monero_p2p::{
    client::InternalPeerID,
    handles::ConnectionHandle,
    services::{PeerSyncRequest, PeerSyncResponse},
    NetworkZone, PeerRequest, PeerResponse, PeerSyncSvc,
};
use monero_pruning::{PruningSeed, CRYPTONOTE_MAX_BLOCK_HEIGHT};
use monero_wire::protocol::{ChainRequest, ChainResponse, GetObjectsRequest};

use crate::{
    client_pool::{ClientPool, ClientPoolDropGuard},
    constants::{
        BLOCK_DOWNLOADER_REQUEST_TIMEOUT, EMPTY_CHAIN_ENTRIES_BEFORE_TOP_ASSUMED,
        INITIAL_CHAIN_REQUESTS_TO_SEND, LONG_BAN, MAX_BLOCKS_IDS_IN_CHAIN_ENTRY,
        MAX_BLOCK_BATCH_LEN, MAX_DOWNLOAD_FAILURES, MAX_TRANSACTION_BLOB_SIZE, MEDIUM_BAN,
    },
};

mod chain_tracker;
use chain_tracker::{BlocksToRetrieve, ChainEntry, ChainTracker};

/// A downloaded batch of blocks.
#[derive(Debug)]
pub struct BlockBatch {
    /// The blocks.
    pub blocks: Vec<(Block, Vec<Transaction>)>,
    /// The size in bytes of this batch.
    pub size: usize,
    /// The peer that gave us this batch.
    pub peer_handle: ConnectionHandle,
}

/// The block downloader config.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq)]
pub struct BlockDownloaderConfig {
    /// The size in bytes of the buffer between the block downloader and the place which
    /// is consuming the downloaded blocks.
    pub buffer_size: usize,
    /// The size of the in progress queue (in bytes) at which we stop requesting more blocks.
    pub in_progress_queue_size: usize,
    /// The [`Duration`] between checking the client pool for free peers.
    pub check_client_pool_interval: Duration,
    /// The target size of a single batch of blocks (in bytes).
    pub target_batch_size: usize,
    /// The initial amount of blocks to request (in number of blocks)
    pub initial_batch_size: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum BlockDownloadError {
    #[error("A request to a peer timed out.")]
    TimedOut,
    #[error("The block buffer was closed.")]
    BufferWasClosed,
    #[error("The peers we requested data from did not have all the data.")]
    PeerDidNotHaveRequestedData,
    #[error("The peers response to a request was invalid.")]
    PeersResponseWasInvalid,
    #[error("The chain we are following is invalid.")]
    ChainInvalid,
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
    /// A request to find the first unknown block ID in a list of block IDs.
    FindFirstUnknown(Vec<[u8; 32]>),
    /// A request for our current cumulative difficulty.
    CumulativeDifficulty,
}

/// The response type for the chain service.
pub enum ChainSvcResponse {
    /// The response for [`ChainSvcRequest::CompactHistory`]
    CompactHistory {
        /// A list of blocks IDs in our chain, starting with the most recent block, all the way to the genesis block.
        ///
        /// These blocks should be in reverse chronological order, not every block is needed.
        block_ids: Vec<[u8; 32]>,
        /// The current cumulative difficulty of the chain.
        cumulative_difficulty: u128,
    },
    /// The response for [`ChainSvcRequest::FindFirstUnknown`], contains the index of the first unknown
    /// block and its expected height.
    FindFirstUnknown(usize, u64),
    /// The current cumulative difficulty of our chain.
    CumulativeDifficulty(u128),
}

/// This function starts the block downloader and returns a [`BufferStream`] that will produce
/// a sequential stream of blocks.
///
/// The block downloader will pick the longest chain and will follow it for as long as possible,
/// the blocks given from the [`BufferStream`] will be in order.
///
/// The block downloader may fail before the whole chain is downloaded. If this is the case you can
/// call this function again, so it can start the search again.
#[instrument(level = "error", skip_all, name = "block_downloader")]
pub fn download_blocks<N: NetworkZone, S, C>(
    client_pool: Arc<ClientPool<N>>,
    peer_sync_svc: S,
    our_chain_svc: C,

    config: BlockDownloaderConfig,
) -> BufferStream<BlockBatch>
where
    S: PeerSyncSvc<N> + Clone,
    C: Service<ChainSvcRequest, Response = ChainSvcResponse, Error = tower::BoxError>
        + Send
        + 'static,
    C::Future: Send + 'static,
{
    let (buffer_appender, buffer_stream) = async_buffer::new_buffer(config.buffer_size);

    let block_downloader = BlockDownloader::new(
        client_pool,
        peer_sync_svc,
        our_chain_svc,
        buffer_appender,
        config,
    );

    tokio::spawn(block_downloader.run().instrument(Span::current()));

    buffer_stream
}

/// A batch of blocks in the ready queue, waiting for previous blocks to come in so they can
/// be passed into the buffer.
#[derive(Debug)]
struct ReadyQueueBatch {
    /// The start height of the batch.
    start_height: u64,
    /// The batch of blocks.
    block_batch: BlockBatch,
}

impl Eq for ReadyQueueBatch {}

impl PartialEq<Self> for ReadyQueueBatch {
    fn eq(&self, other: &Self) -> bool {
        self.start_height.eq(&other.start_height)
    }
}

impl PartialOrd<Self> for ReadyQueueBatch {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ReadyQueueBatch {
    fn cmp(&self, other: &Self) -> Ordering {
        // reverse the ordering so older blocks (lower height) come first in a [`BinaryHeap`]
        self.start_height.cmp(&other.start_height).reverse()
    }
}

/// # Block Downloader
///
/// This is the block downloader, which finds a chain to follow and attempts to follow it, adding the
/// downloaded blocks to an [`async_buffer`].
///
/// ## Implementation Details
///
/// The first step to downloading blocks is to find a chain to follow, this is done by [`initial_chain_search`],
/// docs can be found on that function for details on how this is done.
///
/// With an initial list of block IDs to follow the block downloader will then look for available peers
/// to download blocks from.
///
/// For each peer we will then allocate a batch of blocks for them to retrieve, as these blocks come in
/// we add them to queue for pushing into the [`async_buffer`], once we have the oldest block downloaded
/// we send it into the buffer, repeating this until the oldest current block is still being downloaded.
///
/// When a peer has finished downloading blocks we add it to our list of ready peers, so it can be used to
/// request more data from.
///
/// Ready peers will either:
/// - download the next batch of blocks
/// - request the next chain entry
/// - download an already requested batch of blocks this might happen due to an error in the previous request
/// or because the queue of ready blocks is too large, so we need the oldest block to clear it.
struct BlockDownloader<N: NetworkZone, S, C> {
    /// The client pool.
    client_pool: Arc<ClientPool<N>>,
    /// Peers that are ready to handle requests.
    pending_peers: BTreeMap<PruningSeed, Vec<ClientPoolDropGuard<N>>>,

    /// The service that holds the peers sync states.
    peer_sync_svc: S,
    /// The service that holds our current chain state.
    our_chain_svc: C,

    /// The amount of blocks to request in the next batch.
    amount_of_blocks_to_request: usize,
    /// The height at which `amount_of_blocks_to_request` was updated.
    amount_of_blocks_to_request_updated_at: u64,

    /// The amount of consecutive empty chain entries we received.
    ///
    /// An empty chain entry means we reached the peers chain tip.
    amount_of_empty_chain_entries: usize,

    /// The running block download tasks.
    ///
    /// Returns:
    /// - The start height of the batch
    /// - A result contains the batch or an error.
    #[allow(clippy::type_complexity)]
    block_download_tasks: JoinSet<(
        u64,
        Result<(ClientPoolDropGuard<N>, BlockBatch), BlockDownloadError>,
    )>,
    /// The running chain entry tasks.
    ///
    /// Returns a result of the chain entry or an error.
    #[allow(clippy::type_complexity)]
    chain_entry_task: JoinSet<Result<(ClientPoolDropGuard<N>, ChainEntry<N>), BlockDownloadError>>,

    /// The current inflight requests.
    ///
    /// This a map of batch start heights to block ids and related information of the batch.
    inflight_requests: BTreeMap<u64, BlocksToRetrieve<N>>,

    /// A queue of ready batches.
    ready_batches: BinaryHeap<ReadyQueueBatch>,
    /// The size, in bytes, of all the batches in `ready_batches`.
    ready_batches_size: usize,

    /// A queue of failed batches start height's that should be retried.
    ///
    /// Wrapped in [`Reverse`] so we prioritize early batches.
    failed_batches: BinaryHeap<Reverse<u64>>,

    /// The [`BufferAppender`] that gives blocks to Cuprate.
    buffer_appender: BufferAppender<BlockBatch>,

    /// The [`BlockDownloaderConfig`].
    config: BlockDownloaderConfig,
}

impl<N: NetworkZone, S, C> BlockDownloader<N, S, C>
where
    S: PeerSyncSvc<N> + Clone,
    C: Service<ChainSvcRequest, Response = ChainSvcResponse, Error = tower::BoxError>
        + Send
        + 'static,
    C::Future: Send + 'static,
{
    /// Creates a new [`BlockDownloader`]
    fn new(
        client_pool: Arc<ClientPool<N>>,

        peer_sync_svc: S,
        our_chain_svc: C,
        buffer_appender: BufferAppender<BlockBatch>,

        config: BlockDownloaderConfig,
    ) -> Self {
        BlockDownloader {
            client_pool,
            pending_peers: BTreeMap::new(),
            peer_sync_svc,
            our_chain_svc,
            amount_of_blocks_to_request: config.initial_batch_size,
            amount_of_blocks_to_request_updated_at: 0,
            amount_of_empty_chain_entries: 0,
            block_download_tasks: JoinSet::new(),
            chain_entry_task: JoinSet::new(),
            inflight_requests: BTreeMap::new(),
            ready_batches: BinaryHeap::new(),
            ready_batches_size: 0,
            failed_batches: BinaryHeap::new(),
            buffer_appender,
            config,
        }
    }

    /// Checks if we can make use of any peers that are currently pending requests.
    async fn check_pending_peers(&mut self, chain_tracker: &mut ChainTracker<N>) {
        tracing::debug!("Checking if we can give any work to pending peers.");

        // HACK: The borrow checker doesn't like the following code if we don't do this.
        let mut pending_peers = mem::take(&mut self.pending_peers);

        for (_, peers) in pending_peers.iter_mut() {
            while let Some(peer) = peers.pop() {
                if peer.info.handle.is_closed() {
                    // Peer has disconnected, drop it.
                    continue;
                }

                if let Some(peer) = self.try_handle_free_client(chain_tracker, peer).await {
                    // This peer is ok however it does not have the data we currently need, this will only happen
                    // because of it's pruning seed so just skip over all peers with this pruning seed.
                    peers.push(peer);
                    break;
                }
            }
        }
        // Make sure the calls to `try_handle_free_client` did not add peers to this.
        assert!(self.pending_peers.is_empty());

        self.pending_peers = pending_peers;
    }

    /// Attempts to send another request for an inflight batch
    ///
    /// This function will find the batch(es) that we are waiting on to clear our ready queue and sends another request
    /// for them.
    ///
    /// Returns the [`ClientPoolDropGuard`] back if it doesn't have the batch according to it's pruning seed.
    async fn request_inflight_batch_again(
        &mut self,
        client: ClientPoolDropGuard<N>,
    ) -> Option<ClientPoolDropGuard<N>> {
        tracing::debug!(
            "Requesting an inflight batch, current ready queue size: {}",
            self.ready_batches_size
        );

        if self.inflight_requests.is_empty() {
            panic!("We need requests inflight to be able to send the request again")
        }

        let oldest_ready_batch = self.ready_batches.peek().unwrap().start_height;

        for (_, in_flight_batch) in self.inflight_requests.range_mut(0..oldest_ready_batch) {
            if in_flight_batch.requests_sent >= 2 {
                continue;
            }

            if !client_has_block_in_range(
                &client.info.pruning_seed,
                in_flight_batch.start_height,
                in_flight_batch.ids.len(),
            ) {
                return Some(client);
            }

            in_flight_batch.requests_sent += 1;

            tracing::debug!(
                "Sending request for batch, total requests sent for batch: {}",
                in_flight_batch.requests_sent
            );

            let ids = in_flight_batch.ids.clone();
            let start_height = in_flight_batch.start_height;

            self.block_download_tasks.spawn(async move {
                (
                    start_height,
                    request_batch_from_peer(client, ids, start_height).await,
                )
            });

            return None;
        }

        tracing::debug!("Could not find an inflight request applicable for this peer.");

        Some(client)
    }

    /// Spawns a task to request blocks from the given peer.
    ///
    /// The batch requested will depend on our current state, failed batches will be prioritised.
    ///
    /// Returns the [`ClientPoolDropGuard`] back if it doesn't have the data we currently need according
    /// to it's pruning seed.
    async fn request_block_batch(
        &mut self,
        chain_tracker: &mut ChainTracker<N>,
        client: ClientPoolDropGuard<N>,
    ) -> Option<ClientPoolDropGuard<N>> {
        tracing::trace!("Using peer to request a batch of blocks.");
        // First look to see if we have any failed requests.
        while let Some(failed_request) = self.failed_batches.peek() {
            // Check if we still have the request that failed - another peer could have completed it after
            // failure.
            if let Some(request) = self.inflight_requests.get(&failed_request.0) {
                // Check if this peer has the blocks according to their pruning seed.
                if client_has_block_in_range(
                    &client.info.pruning_seed,
                    request.start_height,
                    request.ids.len(),
                ) {
                    tracing::debug!("Using peer to request a failed batch");
                    // They should have the blocks so send the re-request to this peer.
                    let ids = request.ids.clone();
                    let start_height = request.start_height;

                    self.block_download_tasks.spawn(async move {
                        (
                            start_height,
                            request_batch_from_peer(client, ids, start_height).await,
                        )
                    });
                    // Remove the failure, we have just handled it.
                    self.failed_batches.pop();

                    return None;
                }

                break;
            } else {
                // We don't have the request in flight so remove the failure.
                self.failed_batches.pop();
            }
        }

        // If our ready queue is too large send duplicate requests for the blocks we are waiting on.
        if self.ready_batches_size >= self.config.in_progress_queue_size {
            return self.request_inflight_batch_again(client).await;
        }

        // No failed requests that we can handle, request some new blocks.

        let Some(mut block_entry_to_get) = chain_tracker
            .blocks_to_get(&client.info.pruning_seed, self.amount_of_blocks_to_request)
        else {
            return Some(client);
        };

        tracing::debug!("Requesting a new batch of blocks");

        block_entry_to_get.requests_sent = 1;
        self.inflight_requests
            .insert(block_entry_to_get.start_height, block_entry_to_get.clone());

        self.block_download_tasks.spawn(async move {
            (
                block_entry_to_get.start_height,
                request_batch_from_peer(
                    client,
                    block_entry_to_get.ids,
                    block_entry_to_get.start_height,
                )
                .await,
            )
        });

        None
    }

    /// Attempts to give work to a free client.
    ///
    /// This function will use our current state to decide if we should send a request for a chain entry
    /// or if we should request a batch of blocks.
    ///
    /// Returns the [`ClientPoolDropGuard`] back if it doesn't have the data we currently need according
    /// to it's pruning seed.
    async fn try_handle_free_client(
        &mut self,
        chain_tracker: &mut ChainTracker<N>,
        client: ClientPoolDropGuard<N>,
    ) -> Option<ClientPoolDropGuard<N>> {
        // We send 2 requests, so if one of them is slow/ doesn't have the next chain we still have a backup.
        if self.chain_entry_task.len() < 2
            // If we have had too many failures then assume the tip has been found so no more chain entries.
            && self.amount_of_empty_chain_entries <= EMPTY_CHAIN_ENTRIES_BEFORE_TOP_ASSUMED
            // Check we have a big buffer of pending block IDs to retrieve, we don't want to be waiting around
            // for a chain entry.
            && chain_tracker.block_requests_queued(self.amount_of_blocks_to_request) < 500
            // Make sure this peer actually has the chain.
            && chain_tracker.should_ask_for_next_chain_entry(&client.info.pruning_seed)
        {
            tracing::debug!("Requesting next chain entry");

            let history = chain_tracker.get_simple_history();

            self.chain_entry_task.spawn(
                async move {
                    timeout(
                        BLOCK_DOWNLOADER_REQUEST_TIMEOUT,
                        request_chain_entry_from_peer(client, history),
                    )
                    .await
                    .map_err(|_| BlockDownloadError::TimedOut)?
                }
                .instrument(Span::current()),
            );

            return None;
        }

        // Request a batch of blocks instead.
        self.request_block_batch(chain_tracker, client).await
    }

    /// Checks the [`ClientPool`] for free peers.
    async fn check_for_free_clients(
        &mut self,
        chain_tracker: &mut ChainTracker<N>,
    ) -> Result<(), BlockDownloadError> {
        tracing::debug!("Checking for free peers");

        // This value might be slightly behind but that's ok.
        let ChainSvcResponse::CumulativeDifficulty(current_cumulative_difficulty) = self
            .our_chain_svc
            .ready()
            .await?
            .call(ChainSvcRequest::CumulativeDifficulty)
            .await?
        else {
            panic!("Chain service returned ")
        };

        let PeerSyncResponse::PeersToSyncFrom(peers) = self
            .peer_sync_svc
            .ready()
            .await?
            .call(PeerSyncRequest::PeersToSyncFrom {
                current_cumulative_difficulty,
                block_needed: None,
            })
            .await?
        else {
            panic!("Chain service returned ")
        };

        tracing::debug!("Response received from peer sync service");

        for client in self.client_pool.borrow_clients(&peers) {
            self.pending_peers
                .entry(client.info.pruning_seed)
                .or_default()
                .push(client);
        }

        self.check_pending_peers(chain_tracker).await;

        Ok(())
    }

    /// Checks if we have batches ready to send down the [`BufferAppender`].
    ///
    /// We guarantee that blocks sent down the buffer are sent in the correct order.
    async fn push_new_blocks(&mut self) -> Result<(), BlockDownloadError> {
        while let Some(ready_batch) = self.ready_batches.peek() {
            // Check if this ready batch's start height is higher than the lowest in flight request.
            // If there is a lower start height in the inflight requests then this is _not_ the next batch
            // to send down the buffer.
            if self
                .inflight_requests
                .first_key_value()
                .is_some_and(|(&lowest_start_height, _)| {
                    ready_batch.start_height > lowest_start_height
                })
            {
                break;
            }

            // Our next ready batch is older (lower height) than the oldest in flight, push it down the
            // buffer.
            let ready_batch = self.ready_batches.pop().unwrap();

            let size = ready_batch.block_batch.size;
            self.ready_batches_size -= size;

            tracing::debug!(
                "Pushing batch to buffer, new ready batches size: {}",
                self.ready_batches_size
            );

            self.buffer_appender
                .send(ready_batch.block_batch, size)
                .await
                .map_err(|_| BlockDownloadError::BufferWasClosed)?;

            // Loops back to check the next oldest ready batch.
        }

        Ok(())
    }

    /// Handles a response to a request to get blocks from a peer.
    async fn handle_download_batch_res(
        &mut self,
        start_height: u64,
        res: Result<(ClientPoolDropGuard<N>, BlockBatch), BlockDownloadError>,
        chain_tracker: &mut ChainTracker<N>,
    ) -> Result<(), BlockDownloadError> {
        tracing::debug!("Handling block download response");

        match res {
            Err(e) => {
                if matches!(e, BlockDownloadError::ChainInvalid) {
                    // If the chain was invalid ban the peer who told us about it and error here to stop the
                    // block downloader.
                    self.inflight_requests.get(&start_height).inspect(|entry| {
                        tracing::warn!(
                            "Received an invalid chain from peer: {}, exiting block downloader (it will be restarted).",
                            entry.peer_who_told_us
                        );
                        entry.peer_who_told_us_handle.ban_peer(LONG_BAN)
                    });

                    return Err(e);
                }

                // Add the request to the failed list.
                if let Some(batch) = self.inflight_requests.get_mut(&start_height) {
                    tracing::debug!("Error downloading batch: {e}");

                    batch.failures += 1;
                    if batch.failures > MAX_DOWNLOAD_FAILURES {
                        tracing::debug!(
                            "Too many errors downloading blocks, stopping the block downloader."
                        );
                        return Err(BlockDownloadError::TimedOut);
                    }

                    self.failed_batches.push(Reverse(start_height))
                }

                Ok(())
            }
            Ok((client, block_batch)) => {
                // Remove the batch from the inflight batches.
                if self.inflight_requests.remove(&start_height).is_none() {
                    tracing::debug!("Already retrieved batch");
                    // If it was already retrieved then there is nothing else to do.
                    self.pending_peers
                        .entry(client.info.pruning_seed)
                        .or_default()
                        .push(client);

                    self.check_pending_peers(chain_tracker).await;

                    return Ok(());
                };

                // If the batch is higher than the last time we updated `amount_of_blocks_to_request`, update it
                // again.
                if start_height > self.amount_of_blocks_to_request_updated_at {
                    self.amount_of_blocks_to_request = calculate_next_block_batch_size(
                        block_batch.size,
                        block_batch.blocks.len(),
                        self.config.target_batch_size,
                    );

                    tracing::debug!(
                        "Updating batch size of new batches, new size: {}",
                        self.amount_of_blocks_to_request
                    );

                    self.amount_of_blocks_to_request_updated_at = start_height;
                }

                // Add the batch to the queue of ready batches.
                self.ready_batches_size += block_batch.size;
                self.ready_batches.push(ReadyQueueBatch {
                    start_height,
                    block_batch,
                });

                // Attempt to push new batches to the buffer.
                self.push_new_blocks().await?;

                self.pending_peers
                    .entry(client.info.pruning_seed)
                    .or_default()
                    .push(client);

                self.check_pending_peers(chain_tracker).await;

                Ok(())
            }
        }
    }

    /// Starts the main loop of the block downloader.
    async fn run(mut self) -> Result<(), BlockDownloadError> {
        let mut chain_tracker = initial_chain_search(
            &self.client_pool,
            self.peer_sync_svc.clone(),
            &mut self.our_chain_svc,
        )
        .await?;

        tracing::info!("Attempting to download blocks from peers, this may take a while.");

        let mut check_client_pool_interval = interval(self.config.check_client_pool_interval);
        check_client_pool_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        self.check_for_free_clients(&mut chain_tracker).await?;

        loop {
            tokio::select! {
                _ = check_client_pool_interval.tick() => {
                    tracing::debug!("Checking client pool for free peers, timer fired.");
                    self.check_for_free_clients(&mut chain_tracker).await?;

                     // If we have no inflight requests, and we have had too many empty chain entries in a row assume the top has been found.
                    if self.inflight_requests.is_empty() && self.amount_of_empty_chain_entries >= EMPTY_CHAIN_ENTRIES_BEFORE_TOP_ASSUMED {
                        tracing::debug!("Failed to find any more chain entries, probably fround the top");
                        return Ok(())
                    }
                }
                Some(res) = self.block_download_tasks.join_next() => {
                    let (start_height, res) = res.expect("Download batch future panicked");

                    self.handle_download_batch_res(start_height, res, &mut chain_tracker).await?;

                    // If we have no inflight requests, and we have had too many empty chain entries in a row assume the top has been found.
                    if self.inflight_requests.is_empty() && self.amount_of_empty_chain_entries >= EMPTY_CHAIN_ENTRIES_BEFORE_TOP_ASSUMED {
                        tracing::debug!("Failed to find any more chain entries, probably fround the top");
                        return Ok(())
                    }
                }
                Some(Ok(res)) = self.chain_entry_task.join_next() => {
                    match res {
                        Ok((client, entry)) => {
                            if chain_tracker.add_entry(entry).is_ok() {
                                tracing::debug!("Successfully added chain entry to chain tracker.");
                                self.amount_of_empty_chain_entries = 0;
                            } else {
                                tracing::debug!("Failed to add incoming chain entry to chain tracker.");
                                self.amount_of_empty_chain_entries += 1;
                            }

                            self.pending_peers
                                .entry(client.info.pruning_seed)
                                .or_default()
                                .push(client);

                            self.check_pending_peers(&mut chain_tracker).await;
                        }
                        Err(_) => self.amount_of_empty_chain_entries += 1
                    }
                }
            }
        }
    }
}

fn client_has_block_in_range(pruning_seed: &PruningSeed, start_height: u64, length: usize) -> bool {
    pruning_seed.has_full_block(start_height, CRYPTONOTE_MAX_BLOCK_HEIGHT)
        && pruning_seed.has_full_block(
            start_height + u64::try_from(length).unwrap(),
            CRYPTONOTE_MAX_BLOCK_HEIGHT,
        )
}

/// Calculates the next amount of blocks to request in a batch.
///
/// Parameters:
/// `previous_batch_size` is the size, in bytes, of the last batch,
/// `previous_batch_len` is the amount of blocks in the last batch,
/// `target_batch_size` is the target size, in bytes, of a batch.
fn calculate_next_block_batch_size(
    previous_batch_size: usize,
    previous_batch_len: usize,
    target_batch_size: usize,
) -> usize {
    // The average block size of the last batch of blocks, multiplied by 2 as a safety margin for
    // future blocks.
    let adjusted_average_block_size = max((previous_batch_size * 2) / previous_batch_len, 1);

    // Set the amount of blocks to request equal to our target batch size divided by the adjusted_average_block_size.
    let next_batch_len = max(target_batch_size / adjusted_average_block_size, 1);

    // Cap the amount of growth to 1.5x the previous batch len, to prevent a small block casing us to request
    // a huge amount of blocks.
    let next_batch_len = min(next_batch_len, (previous_batch_len * 3).div_ceil(2));

    // Cap the length to the maximum allowed.
    min(next_batch_len, MAX_BLOCK_BATCH_LEN)
}

/// Requests a sequential batch of blocks from a peer.
///
/// This function will validate the blocks that were downloaded were the ones asked for and that they match
/// the expected height.
async fn request_batch_from_peer<N: NetworkZone>(
    mut client: ClientPoolDropGuard<N>,
    ids: ByteArrayVec<32>,
    expected_start_height: u64,
) -> Result<(ClientPoolDropGuard<N>, BlockBatch), BlockDownloadError> {
    // Request the blocks.
    let blocks_response = timeout(BLOCK_DOWNLOADER_REQUEST_TIMEOUT, async {
        let PeerResponse::GetObjects(blocks_response) = client
            .ready()
            .await?
            .call(PeerRequest::GetObjects(GetObjectsRequest {
                blocks: ids.clone(),
                pruned: false,
            }))
            .await?
        else {
            panic!("Connection task returned wrong response.");
        };

        Ok::<_, BlockDownloadError>(blocks_response)
    })
    .await
    .map_err(|_| BlockDownloadError::TimedOut)??;

    // Initial sanity checks
    if blocks_response.blocks.len() > ids.len() {
        client.info.handle.ban_peer(MEDIUM_BAN);
        return Err(BlockDownloadError::PeersResponseWasInvalid);
    }

    if blocks_response.blocks.len() != ids.len() {
        return Err(BlockDownloadError::PeerDidNotHaveRequestedData);
    }

    let blocks = rayon_spawn_async(move || {
        let blocks = blocks_response
            .blocks
            .into_par_iter()
            .enumerate()
            .map(|(i, block_entry)| {
                let expected_height = u64::try_from(i).unwrap() + expected_start_height;

                let mut size = block_entry.block.len();

                let block = Block::read(&mut block_entry.block.as_ref())
                    .map_err(|_| BlockDownloadError::PeersResponseWasInvalid)?;

                // Check the block matches the one requested and the peer sent enough transactions.
                if ids[i] != block.hash() || block.txs.len() != block_entry.txs.len() {
                    return Err(BlockDownloadError::PeersResponseWasInvalid);
                }

                // Check the height lines up as expected.
                // This must happen after the hash check.
                if !block
                    .number()
                    .is_some_and(|height| height == expected_height)
                {
                    // This peer probably did nothing wrong, it was the peer who told us this blockID which
                    // is misbehaving.
                    return Err(BlockDownloadError::ChainInvalid);
                }

                // Deserialize the transactions.
                let txs = block_entry
                    .txs
                    .take_normal()
                    .ok_or(BlockDownloadError::PeersResponseWasInvalid)?
                    .into_iter()
                    .map(|tx_blob| {
                        size += tx_blob.len();

                        if tx_blob.len() > MAX_TRANSACTION_BLOB_SIZE {
                            return Err(BlockDownloadError::PeersResponseWasInvalid);
                        }

                        Transaction::read(&mut tx_blob.as_ref())
                            .map_err(|_| BlockDownloadError::PeersResponseWasInvalid)
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                // Make sure the transactions in the block were the ones the peer sent.
                let mut expected_txs = block.txs.iter().collect::<HashSet<_>>();

                for tx in &txs {
                    if !expected_txs.remove(&tx.hash()) {
                        return Err(BlockDownloadError::PeersResponseWasInvalid);
                    }
                }

                if !expected_txs.is_empty() {
                    return Err(BlockDownloadError::PeersResponseWasInvalid);
                }

                Ok(((block, txs), size))
            })
            .collect::<Result<(Vec<_>, Vec<_>), _>>();

        blocks
    })
    .await;

    let (blocks, sizes) = blocks.inspect_err(|e| {
        // If the peers response was invalid, ban it.
        if matches!(e, BlockDownloadError::PeersResponseWasInvalid) {
            client.info.handle.ban_peer(MEDIUM_BAN);
        }
    })?;

    let peer_handle = client.info.handle.clone();

    Ok((
        client,
        BlockBatch {
            blocks,
            size: sizes.iter().sum(),
            peer_handle,
        },
    ))
}

/// Request a chain entry from a peer.
///
/// Because the block downloader only follows and downloads one chain we only have to send the block hash of
/// top block we have found and the genesis block, this is then called `short_history`.
async fn request_chain_entry_from_peer<N: NetworkZone>(
    mut client: ClientPoolDropGuard<N>,
    short_history: [[u8; 32]; 2],
) -> Result<(ClientPoolDropGuard<N>, ChainEntry<N>), BlockDownloadError> {
    let PeerResponse::GetChain(chain_res) = client
        .ready()
        .await?
        .call(PeerRequest::GetChain(ChainRequest {
            block_ids: short_history.into(),
            prune: true,
        }))
        .await?
    else {
        panic!("Connection task returned wrong response!");
    };

    if chain_res.m_block_ids.is_empty()
        || chain_res.m_block_ids.len() > MAX_BLOCKS_IDS_IN_CHAIN_ENTRY
    {
        client.info.handle.ban_peer(MEDIUM_BAN);
        return Err(BlockDownloadError::PeersResponseWasInvalid);
    }

    // We must have at least one overlapping block.
    if !(chain_res.m_block_ids[0] == short_history[0]
        || chain_res.m_block_ids[0] == short_history[1])
    {
        client.info.handle.ban_peer(MEDIUM_BAN);
        return Err(BlockDownloadError::PeersResponseWasInvalid);
    }

    // If the genesis is the overlapping block then this peer does not have our top tracked block in
    // its chain.
    if chain_res.m_block_ids[0] == short_history[1] {
        return Err(BlockDownloadError::PeerDidNotHaveRequestedData);
    }

    let entry = ChainEntry {
        ids: (&chain_res.m_block_ids).into(),
        peer: client.info.id,
        handle: client.info.handle.clone(),
    };

    Ok((client, entry))
}

/// Initial chain search, this function pulls [`INITIAL_CHAIN_REQUESTS_TO_SEND`] peers from the [`ClientPool`]
/// and sends chain requests to all of them.
///
/// We then wait for their response and choose the peer who claims the highest cumulative difficulty.
#[instrument(level = "error", skip_all)]
async fn initial_chain_search<N: NetworkZone, S, C>(
    client_pool: &Arc<ClientPool<N>>,
    mut peer_sync_svc: S,
    mut our_chain_svc: C,
) -> Result<ChainTracker<N>, BlockDownloadError>
where
    S: PeerSyncSvc<N>,
    C: Service<ChainSvcRequest, Response = ChainSvcResponse, Error = tower::BoxError>,
{
    tracing::debug!("Getting our chain history");
    // Get our history.
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

    tracing::debug!("Getting a list of peers with higher cumulative difficulty");

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

    tracing::debug!(
        "{} peers claim they have a higher cumulative difficulty",
        peers.len()
    );

    // Shuffle the list to remove any possibility of peers being able to prioritize getting picked.
    peers.shuffle(&mut thread_rng());

    let mut peers = client_pool.borrow_clients(&peers);

    let mut futs = JoinSet::new();

    let req = PeerRequest::GetChain(ChainRequest {
        block_ids: block_ids.into(),
        prune: false,
    });

    tracing::debug!("Sending requests for chain entries.");

    // Send the requests.
    while futs.len() < INITIAL_CHAIN_REQUESTS_TO_SEND {
        let Some(mut next_peer) = peers.next() else {
            break;
        };

        let cloned_req = req.clone();
        futs.spawn(timeout(
            BLOCK_DOWNLOADER_REQUEST_TIMEOUT,
            async move {
                let PeerResponse::GetChain(chain_res) =
                    next_peer.ready().await?.call(cloned_req).await?
                else {
                    panic!("connection task returned wrong response!");
                };

                Ok::<_, tower::BoxError>((
                    chain_res,
                    next_peer.info.id,
                    next_peer.info.handle.clone(),
                ))
            }
            .instrument(Span::current()),
        ));
    }

    let mut res: Option<(ChainResponse, InternalPeerID<_>, ConnectionHandle)> = None;

    // Wait for the peers responses.
    while let Some(task_res) = futs.join_next().await {
        let Ok(Ok(task_res)) = task_res.unwrap() else {
            continue;
        };

        match &mut res {
            Some(res) => {
                // res has already been set, replace it if this peer claims higher cumulative difficulty
                if res.0.cumulative_difficulty() < task_res.0.cumulative_difficulty() {
                    let _ = mem::replace(res, task_res);
                }
            }
            None => {
                // res has not been set, set it now;
                res = Some(task_res);
            }
        }
    }

    let Some((chain_res, peer_id, peer_handle)) = res else {
        return Err(BlockDownloadError::FailedToFindAChainToFollow);
    };

    let hashes: Vec<[u8; 32]> = (&chain_res.m_block_ids).into();
    // drop this to deallocate the [`Bytes`].
    drop(chain_res);

    tracing::debug!("Highest chin entry contained {} block Ids", hashes.len());

    // Find the first unknown block in the batch.
    let ChainSvcResponse::FindFirstUnknown(first_unknown, expected_height) = our_chain_svc
        .ready()
        .await?
        .call(ChainSvcRequest::FindFirstUnknown(hashes.clone()))
        .await?
    else {
        panic!("chain service sent wrong response.");
    };

    // The peer must send at least one block we already know.
    if first_unknown == 0 {
        peer_handle.ban_peer(MEDIUM_BAN);
        return Err(BlockDownloadError::PeerSentNoOverlappingBlocks);
    }

    // We know all the blocks already
    // TODO: The peer could still be on a different chain, however the chain might just be too far split.
    if first_unknown == hashes.len() {
        return Err(BlockDownloadError::FailedToFindAChainToFollow);
    }

    let first_entry = ChainEntry {
        ids: hashes[first_unknown..].to_vec(),
        peer: peer_id,
        handle: peer_handle,
    };

    tracing::debug!(
        "Creating chain tracker with {} new block Ids",
        first_entry.ids.len()
    );

    let tracker = ChainTracker::new(first_entry, expected_height, our_genesis);

    Ok(tracker)
}

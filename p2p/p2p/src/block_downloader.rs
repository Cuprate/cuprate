//! # Block Downloader
//!
//! This module contains the [`BlockDownloader`], which finds a chain to
//! download from our connected peers and downloads it. See the actual
//! `struct` documentation for implementation details.
//!
//! The block downloader is started by [`download_blocks`].
use std::{
    cmp::{max, min, Reverse},
    collections::{BTreeMap, BinaryHeap},
    sync::Arc,
    time::Duration,
};

use futures::TryFutureExt;
use monero_serai::{block::Block, transaction::Transaction};
use tokio::{
    task::JoinSet,
    time::{interval, timeout, MissedTickBehavior},
};
use tower::{Service, ServiceExt};
use tracing::{instrument, Instrument, Span};

use cuprate_async_buffer::{BufferAppender, BufferStream};
use cuprate_p2p_core::{handles::ConnectionHandle, NetworkZone};
use cuprate_pruning::{PruningSeed, CRYPTONOTE_MAX_BLOCK_HEIGHT};

use crate::{
    client_pool::{ClientPool, ClientPoolDropGuard},
    constants::{
        BLOCK_DOWNLOADER_REQUEST_TIMEOUT, EMPTY_CHAIN_ENTRIES_BEFORE_TOP_ASSUMED, LONG_BAN,
        MAX_BLOCK_BATCH_LEN, MAX_DOWNLOAD_FAILURES,
    },
};

mod block_queue;
mod chain_tracker;
mod download_batch;
mod request_chain;
#[cfg(test)]
mod tests;

use block_queue::{BlockQueue, ReadyQueueBatch};
use chain_tracker::{BlocksToRetrieve, ChainEntry, ChainTracker};
use download_batch::download_batch_task;
use request_chain::{initial_chain_search, request_chain_entry_from_peer};

/// A downloaded batch of blocks.
#[derive(Debug, Clone)]
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

/// An error that occurred in the [`BlockDownloader`].
#[derive(Debug, thiserror::Error)]
pub(crate) enum BlockDownloadError {
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
    /// The response for [`ChainSvcRequest::CompactHistory`].
    CompactHistory {
        /// A list of blocks IDs in our chain, starting with the most recent block, all the way to the genesis block.
        ///
        /// These blocks should be in reverse chronological order, not every block is needed.
        block_ids: Vec<[u8; 32]>,
        /// The current cumulative difficulty of the chain.
        cumulative_difficulty: u128,
    },
    /// The response for [`ChainSvcRequest::FindFirstUnknown`].
    ///
    /// Contains the index of the first unknown block and its expected height.
    FindFirstUnknown(Option<(usize, usize)>),
    /// The response for [`ChainSvcRequest::CumulativeDifficulty`].
    ///
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
pub fn download_blocks<N: NetworkZone, C>(
    client_pool: Arc<ClientPool<N>>,
    our_chain_svc: C,
    config: BlockDownloaderConfig,
) -> BufferStream<BlockBatch>
where
    C: Service<ChainSvcRequest, Response = ChainSvcResponse, Error = tower::BoxError>
        + Send
        + 'static,
    C::Future: Send + 'static,
{
    let (buffer_appender, buffer_stream) = cuprate_async_buffer::new_buffer(config.buffer_size);

    let block_downloader =
        BlockDownloader::new(client_pool, our_chain_svc, buffer_appender, config);

    tokio::spawn(
        block_downloader
            .run()
            .inspect_err(|e| tracing::debug!("Error downloading blocks: {e}"))
            .instrument(Span::current()),
    );

    buffer_stream
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
/// we add them to the [`BlockQueue`] for pushing into the [`async_buffer`], once we have the oldest block downloaded
/// we send it into the buffer, repeating this until the oldest current block is still being downloaded.
///
/// When a peer has finished downloading blocks we add it to our list of ready peers, so it can be used to
/// request more data from.
///
/// Ready peers will either:
/// - download the next batch of blocks
/// - request the next chain entry
/// - download an already requested batch of blocks (this might happen due to an error in the previous request
///   or because the queue of ready blocks is too large, so we need the oldest block to clear it).
struct BlockDownloader<N: NetworkZone, C> {
    /// The client pool.
    client_pool: Arc<ClientPool<N>>,

    /// The service that holds our current chain state.
    our_chain_svc: C,

    /// The amount of blocks to request in the next batch.
    amount_of_blocks_to_request: usize,
    /// The height at which [`Self::amount_of_blocks_to_request`] was updated.
    amount_of_blocks_to_request_updated_at: usize,

    /// The amount of consecutive empty chain entries we received.
    ///
    /// An empty chain entry means we reached the peer's chain tip.
    amount_of_empty_chain_entries: usize,

    /// The running block download tasks.
    block_download_tasks: JoinSet<BlockDownloadTaskResponse<N>>,
    /// The running chain entry tasks.
    ///
    /// Returns a result of the chain entry or an error.
    #[expect(clippy::type_complexity)]
    chain_entry_task: JoinSet<Result<(ClientPoolDropGuard<N>, ChainEntry<N>), BlockDownloadError>>,

    /// The current inflight requests.
    ///
    /// This is a map of batch start heights to block IDs and related information of the batch.
    inflight_requests: BTreeMap<usize, BlocksToRetrieve<N>>,

    /// A queue of start heights from failed batches that should be retried.
    ///
    /// Wrapped in [`Reverse`] so we prioritize early batches.
    failed_batches: BinaryHeap<Reverse<usize>>,

    block_queue: BlockQueue,

    /// The [`BlockDownloaderConfig`].
    config: BlockDownloaderConfig,
}

impl<N: NetworkZone, C> BlockDownloader<N, C>
where
    C: Service<ChainSvcRequest, Response = ChainSvcResponse, Error = tower::BoxError>
        + Send
        + 'static,
    C::Future: Send + 'static,
{
    /// Creates a new [`BlockDownloader`]
    fn new(
        client_pool: Arc<ClientPool<N>>,
        our_chain_svc: C,
        buffer_appender: BufferAppender<BlockBatch>,
        config: BlockDownloaderConfig,
    ) -> Self {
        Self {
            client_pool,
            our_chain_svc,
            amount_of_blocks_to_request: config.initial_batch_size,
            amount_of_blocks_to_request_updated_at: 0,
            amount_of_empty_chain_entries: 0,
            block_download_tasks: JoinSet::new(),
            chain_entry_task: JoinSet::new(),
            inflight_requests: BTreeMap::new(),
            block_queue: BlockQueue::new(buffer_appender),
            failed_batches: BinaryHeap::new(),
            config,
        }
    }

    /// Checks if we can make use of any peers that are currently pending requests.
    fn check_pending_peers(
        &mut self,
        chain_tracker: &mut ChainTracker<N>,
        pending_peers: &mut BTreeMap<PruningSeed, Vec<ClientPoolDropGuard<N>>>,
    ) {
        tracing::debug!("Checking if we can give any work to pending peers.");

        for (_, peers) in pending_peers.iter_mut() {
            while let Some(peer) = peers.pop() {
                if peer.info.handle.is_closed() {
                    // Peer has disconnected, drop it.
                    continue;
                }

                let client = self.try_handle_free_client(chain_tracker, peer);
                if let Some(peer) = client {
                    // This peer is ok however it does not have the data we currently need, this will only happen
                    // because of its pruning seed so just skip over all peers with this pruning seed.
                    peers.push(peer);
                    break;
                }
            }
        }
    }

    /// Attempts to send another request for an inflight batch
    ///
    /// This function will find the batch(es) that we are waiting on to clear our ready queue and sends another request
    /// for them.
    ///
    /// Returns the [`ClientPoolDropGuard`] back if it doesn't have the batch according to its pruning seed.
    fn request_inflight_batch_again(
        &mut self,
        client: ClientPoolDropGuard<N>,
    ) -> Option<ClientPoolDropGuard<N>> {
        tracing::debug!(
            "Requesting an inflight batch, current ready queue size: {}",
            self.block_queue.size()
        );

        assert!(
            !self.inflight_requests.is_empty(),
            "We need requests inflight to be able to send the request again",
        );

        let oldest_ready_batch = self.block_queue.oldest_ready_batch().unwrap();

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

            self.block_download_tasks.spawn(download_batch_task(
                client,
                in_flight_batch.ids.clone(),
                in_flight_batch.prev_id,
                in_flight_batch.start_height,
                in_flight_batch.requests_sent,
            ));

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
    /// to its pruning seed.
    fn request_block_batch(
        &mut self,
        chain_tracker: &mut ChainTracker<N>,
        client: ClientPoolDropGuard<N>,
    ) -> Option<ClientPoolDropGuard<N>> {
        tracing::trace!("Using peer to request a batch of blocks.");
        // First look to see if we have any failed requests.
        while let Some(failed_request) = self.failed_batches.peek() {
            // Check if we still have the request that failed - another peer could have completed it after
            // failure.
            let Some(request) = self.inflight_requests.get_mut(&failed_request.0) else {
                // We don't have the request in flight so remove the failure.
                self.failed_batches.pop();
                continue;
            };
            // Check if this peer has the blocks according to their pruning seed.
            if client_has_block_in_range(
                &client.info.pruning_seed,
                request.start_height,
                request.ids.len(),
            ) {
                tracing::debug!("Using peer to request a failed batch");
                // They should have the blocks so send the re-request to this peer.

                request.requests_sent += 1;

                self.block_download_tasks.spawn(download_batch_task(
                    client,
                    request.ids.clone(),
                    request.prev_id,
                    request.start_height,
                    request.requests_sent,
                ));

                // Remove the failure, we have just handled it.
                self.failed_batches.pop();

                return None;
            }
            // The peer doesn't have the batch according to its pruning seed.
            break;
        }

        // If our ready queue is too large send duplicate requests for the blocks we are waiting on.
        if self.block_queue.size() >= self.config.in_progress_queue_size {
            return self.request_inflight_batch_again(client);
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

        self.block_download_tasks.spawn(download_batch_task(
            client,
            block_entry_to_get.ids.clone(),
            block_entry_to_get.prev_id,
            block_entry_to_get.start_height,
            block_entry_to_get.requests_sent,
        ));

        None
    }

    /// Attempts to give work to a free client.
    ///
    /// This function will use our current state to decide if we should send a request for a chain entry
    /// or if we should request a batch of blocks.
    ///
    /// Returns the [`ClientPoolDropGuard`] back if it doesn't have the data we currently need according
    /// to its pruning seed.
    fn try_handle_free_client(
        &mut self,
        chain_tracker: &mut ChainTracker<N>,
        client: ClientPoolDropGuard<N>,
    ) -> Option<ClientPoolDropGuard<N>> {
        // We send 2 requests, so if one of them is slow or doesn't have the next chain, we still have a backup.
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
                .instrument(tracing::debug_span!(
                    "request_chain_entry",
                    current_height = chain_tracker.top_height()
                )),
            );

            return None;
        }

        // Request a batch of blocks instead.
        self.request_block_batch(chain_tracker, client)
    }

    /// Checks the [`ClientPool`] for free peers.
    async fn check_for_free_clients(
        &mut self,
        chain_tracker: &mut ChainTracker<N>,
        pending_peers: &mut BTreeMap<PruningSeed, Vec<ClientPoolDropGuard<N>>>,
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
            panic!("Chain service returned wrong response.");
        };

        for client in self
            .client_pool
            .clients_with_more_cumulative_difficulty(current_cumulative_difficulty)
        {
            pending_peers
                .entry(client.info.pruning_seed)
                .or_default()
                .push(client);
        }

        self.check_pending_peers(chain_tracker, pending_peers);

        Ok(())
    }

    /// Handles a response to a request to get blocks from a peer.
    async fn handle_download_batch_res(
        &mut self,
        start_height: usize,
        res: Result<(ClientPoolDropGuard<N>, BlockBatch), BlockDownloadError>,
        chain_tracker: &mut ChainTracker<N>,
        pending_peers: &mut BTreeMap<PruningSeed, Vec<ClientPoolDropGuard<N>>>,
    ) -> Result<(), BlockDownloadError> {
        tracing::debug!("Handling block download response");

        match res {
            Err(e) => {
                if matches!(e, BlockDownloadError::ChainInvalid) {
                    // If the chain was invalid ban the peer who told us about it and error here to stop the
                    // block downloader.
                    self.inflight_requests.get(&start_height).inspect(|entry| {
                        tracing::warn!(
                            "Received an invalid chain from peer: {}, exiting block downloader (it should be restarted).",
                            entry.peer_who_told_us
                        );
                        entry.peer_who_told_us_handle.ban_peer(LONG_BAN);
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

                    self.failed_batches.push(Reverse(start_height));
                }

                Ok(())
            }
            Ok((client, block_batch)) => {
                // Remove the batch from the inflight batches.
                if self.inflight_requests.remove(&start_height).is_none() {
                    tracing::debug!("Already retrieved batch");
                    // If it was already retrieved then there is nothing else to do.
                    pending_peers
                        .entry(client.info.pruning_seed)
                        .or_default()
                        .push(client);

                    self.check_pending_peers(chain_tracker, pending_peers);

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

                self.block_queue
                    .add_incoming_batch(
                        ReadyQueueBatch {
                            start_height,
                            block_batch,
                        },
                        self.inflight_requests.first_key_value().map(|(k, _)| *k),
                    )
                    .await?;

                pending_peers
                    .entry(client.info.pruning_seed)
                    .or_default()
                    .push(client);

                self.check_pending_peers(chain_tracker, pending_peers);

                Ok(())
            }
        }
    }

    /// Starts the main loop of the block downloader.
    async fn run(mut self) -> Result<(), BlockDownloadError> {
        let mut chain_tracker =
            initial_chain_search(&self.client_pool, &mut self.our_chain_svc).await?;

        let mut pending_peers = BTreeMap::new();

        tracing::info!("Attempting to download blocks from peers, this may take a while.");

        let mut check_client_pool_interval = interval(self.config.check_client_pool_interval);
        check_client_pool_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        self.check_for_free_clients(&mut chain_tracker, &mut pending_peers)
            .await?;

        loop {
            tokio::select! {
                _ = check_client_pool_interval.tick() => {
                    tracing::debug!("Checking client pool for free peers, timer fired.");
                    self.check_for_free_clients(&mut chain_tracker, &mut pending_peers).await?;

                     // If we have no inflight requests, and we have had too many empty chain entries in a row assume the top has been found.
                    if self.inflight_requests.is_empty() && self.amount_of_empty_chain_entries >= EMPTY_CHAIN_ENTRIES_BEFORE_TOP_ASSUMED {
                        tracing::debug!("Failed to find any more chain entries, probably fround the top");
                        return Ok(());
                    }
                }
                Some(res) = self.block_download_tasks.join_next() => {
                    let BlockDownloadTaskResponse {
                        start_height,
                        result
                    } = res.expect("Download batch future panicked");

                    self.handle_download_batch_res(start_height, result, &mut chain_tracker, &mut pending_peers).await?;

                    // If we have no inflight requests, and we have had too many empty chain entries in a row assume the top has been found.
                    if self.inflight_requests.is_empty() && self.amount_of_empty_chain_entries >= EMPTY_CHAIN_ENTRIES_BEFORE_TOP_ASSUMED {
                        tracing::debug!("Failed to find any more chain entries, probably fround the top");
                        return Ok(());
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

                            pending_peers
                                .entry(client.info.pruning_seed)
                                .or_default()
                                .push(client);

                            self.check_pending_peers(&mut chain_tracker, &mut pending_peers);
                        }
                        Err(_) => self.amount_of_empty_chain_entries += 1
                    }
                }
            }
        }
    }
}

/// The return value from the block download tasks.
struct BlockDownloadTaskResponse<N: NetworkZone> {
    /// The start height of the batch.
    start_height: usize,
    /// A result containing the batch or an error.
    result: Result<(ClientPoolDropGuard<N>, BlockBatch), BlockDownloadError>,
}

/// Returns if a peer has all the blocks in a range, according to its [`PruningSeed`].
const fn client_has_block_in_range(
    pruning_seed: &PruningSeed,
    start_height: usize,
    length: usize,
) -> bool {
    pruning_seed.has_full_block(start_height, CRYPTONOTE_MAX_BLOCK_HEIGHT)
        && pruning_seed.has_full_block(start_height + length, CRYPTONOTE_MAX_BLOCK_HEIGHT)
}

/// Calculates the next amount of blocks to request in a batch.
///
/// Parameters:
/// - `previous_batch_size` is the size, in bytes, of the last batch
/// - `previous_batch_len` is the amount of blocks in the last batch
/// - `target_batch_size` is the target size, in bytes, of a batch
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

    // Cap the amount of growth to 1.5x the previous batch len, to prevent a small block causing us to request
    // a huge amount of blocks.
    let next_batch_len = min(next_batch_len, (previous_batch_len * 3).div_ceil(2));

    // Cap the length to the maximum allowed.
    min(next_batch_len, MAX_BLOCK_BATCH_LEN)
}

//! # Block Downloader
//!
use std::{
    cmp::{max, min, Ordering, Reverse},
    collections::{BTreeMap, BinaryHeap, HashSet},
    sync::Arc,
    time::Duration,
};

use monero_serai::{block::Block, transaction::Transaction};
use rand::prelude::*;
use rayon::prelude::*;
use tokio::time::timeout;
use tokio::{
    task::JoinSet,
    time::{interval, MissedTickBehavior},
};
use tower::{Service, ServiceExt};

use async_buffer::{BufferAppender, BufferStream};
use cuprate_helper::asynch::rayon_spawn_async;
use fixed_bytes::ByteArrayVec;
use monero_p2p::{
    client::InternalPeerID,
    handles::ConnectionHandle,
    services::{PeerSyncRequest, PeerSyncResponse},
    NetworkZone, PeerRequest, PeerResponse, PeerSyncSvc,
};
use monero_pruning::CRYPTONOTE_MAX_BLOCK_HEIGHT;
use monero_wire::protocol::{ChainRequest, ChainResponse, GetObjectsRequest};

use crate::{
    client_pool::{ClientPool, ClientPoolDropGuard},
    constants::{INITIAL_CHAIN_REQUESTS_TO_SEND, LONG_BAN, MEDIUM_BAN},
};

mod chain_tracker;
use crate::constants::{CHIAN_ENTRY_REQUEST_TIMEOUT, MAX_BLOCK_BATCH_LEN};
use chain_tracker::{BlocksToRetrieve, ChainEntry, ChainTracker};

/// A downloaded batch of blocks.
#[derive(Debug)]
pub struct BlockBatch {
    /// The blocks.
    pub blocks: Vec<(Block, Vec<Transaction>)>,
    /// The size of this batch in bytes.
    pub size: usize,
    /// The peer that gave us this block.
    pub peer_handle: ConnectionHandle,
}

/// The block downloader config.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq)]
pub struct BlockDownloaderConfig {
    /// The size of the buffer between the block downloader and the place which
    /// is consuming the downloaded blocks.
    pub buffer_size: usize,
    /// The size of the in progress queue at which we stop requesting more blocks.
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
    /// A request to find the first unknown
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
        /// These blocks should be in reverse chronological order and not every block is needed.
        block_ids: Vec<[u8; 32]>,
        cumulative_difficulty: u128,
    },
    /// The response for [`ChainSvcRequest::FindFirstUnknown`], contains the index of the first unknown
    /// block.
    FindFirstUnknown(usize),
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

    tokio::spawn(block_downloader.run());

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
        // reverse the ordering so earlier blocks come first in a [`BinaryHeap`]
        self.start_height.cmp(&other.start_height).reverse()
    }
}

/// # Block Downloader
///
/// This is the block downloader, which finds a chain to follow and attempts to follow it adding the blocks
/// to a [`async_buffer`].
struct BlockDownloader<N: NetworkZone, S, C> {
    /// The client pool.
    client_pool: Arc<ClientPool<N>>,

    /// The service that holds the peers sync states.
    peer_sync_svc: S,
    /// The service that holds our current chain state.
    our_chain_svc: C,

    /// The amount of blocks to request in the next batch.
    amount_of_blocks_to_request: usize,
    /// The height at which `amount_of_blocks_to_request` was updated.
    amount_of_blocks_to_request_updated_at: u64,

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
    fn new(
        client_pool: Arc<ClientPool<N>>,

        peer_sync_svc: S,
        our_chain_svc: C,
        buffer_appender: BufferAppender<BlockBatch>,

        config: BlockDownloaderConfig,
    ) -> Self {
        BlockDownloader {
            client_pool,
            peer_sync_svc,
            our_chain_svc,
            amount_of_blocks_to_request: config.initial_batch_size,
            amount_of_blocks_to_request_updated_at: 0,
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

    async fn request_inflight_batch_again(&mut self, client: ClientPoolDropGuard<N>) {
        if self.inflight_requests.is_empty() {
            panic!("We need requests inflight to be able to send the request again")
        }

        let first_batch_requests_sent = self
            .inflight_requests
            .first_key_value()
            .unwrap()
            .1
            .requests_sent;

        if first_batch_requests_sent
            == self
                .inflight_requests
                .last_key_value()
                .unwrap()
                .1
                .requests_sent
        {
            let mut first_batch = self.inflight_requests.first_entry().unwrap();

            first_batch.get_mut().requests_sent += 1;

            // They should have the blocks so send the re-request to this peer.
            let ids = first_batch.get().ids.clone();
            let start_height = first_batch.get().start_height;

            self.block_download_tasks.spawn(async move {
                (
                    start_height,
                    request_batch_from_peer(client, ids, start_height).await,
                )
            });

            return;
        }

        let next_batch = self
            .inflight_requests
            .iter_mut()
            .find(|(_, next_batch)| next_batch.requests_sent != first_batch_requests_sent)
            .unwrap()
            .1;

        next_batch.requests_sent += 1;

        // They should have the blocks so send the re-request to this peer.
        let ids = next_batch.ids.clone();
        let start_height = next_batch.start_height;

        self.block_download_tasks.spawn(async move {
            (
                start_height,
                request_batch_from_peer(client, ids, start_height).await,
            )
        });
    }

    /// Spawns a task to request blocks from the given peer.
    async fn request_block_batch(
        &mut self,
        chain_tracker: &mut ChainTracker<N>,
        client: ClientPoolDropGuard<N>,
    ) {
        // First look to see if we have any failed requests.
        while let Some(failed_request) = self.failed_batches.peek() {
            // Check if we still have the request that failed - another peer could have completed it after
            // failure.
            if let Some(request) = self.inflight_requests.get(&failed_request.0) {
                // Check if this peer has the blocks according to their pruning seed.
                if client
                    .info
                    .pruning_seed
                    .has_full_block(request.start_height, CRYPTONOTE_MAX_BLOCK_HEIGHT)
                    && client.info.pruning_seed.has_full_block(
                        request.start_height + u64::try_from(request.ids.len()).unwrap(),
                        CRYPTONOTE_MAX_BLOCK_HEIGHT,
                    )
                {
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

                    return;
                }

                break;
            } else {
                // We don't have the request in flight so remove the failure.
                self.failed_batches.pop();
            }
        }

        if self.ready_batches_size >= self.config.in_progress_queue_size {
            self.request_inflight_batch_again(client).await;
            return;
        }

        // No failed requests that we can handle, request some new blocks.

        let Some(block_entry_to_get) = chain_tracker
            .blocks_to_get(&client.info.pruning_seed, self.amount_of_blocks_to_request)
        else {
            return;
        };

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
    }

    async fn handle_free_client(
        &mut self,
        chain_tracker: &mut ChainTracker<N>,
        client: ClientPoolDropGuard<N>,
    ) {
        if self.chain_entry_task.len() < 2
            && chain_tracker.block_requests_queued(self.amount_of_blocks_to_request) < 500
            && chain_tracker.should_ask_for_next_chain_entry(&client.info.pruning_seed)
        {
            let history = chain_tracker.get_simple_history();

            self.chain_entry_task.spawn(async move {
                timeout(
                    CHIAN_ENTRY_REQUEST_TIMEOUT,
                    request_chain_entry_from_peer(client, history),
                )
                .await
                .map_err(|_| BlockDownloadError::TimedOut)?
            });

            return;
        }

        self.request_block_batch(chain_tracker, client).await;
    }

    async fn check_for_free_clients(
        &mut self,
        chain_tracker: &mut ChainTracker<N>,
    ) -> Result<(), BlockDownloadError> {
        tracing::debug!("Checking for free peers");

        // This value might be slightly behind but thats ok.
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

        // Rust borrow rules mean we have to build a vec here.
        let mut clients = Vec::with_capacity(peers.len());
        clients.extend(self.client_pool.borrow_clients(&peers));

        for peer in clients {
            self.handle_free_client(chain_tracker, peer).await;
        }

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
        match res {
            Err(e) => {
                // TODO: (IMPORTANT) check if this failure is from the peer who told us about the batch, if so ban them.

                if matches!(e, BlockDownloadError::ChainInvalid) {
                    // If the chain was invalid ban the peer who told us about it.
                    self.inflight_requests
                        .get(&start_height)
                        .inspect(|entry| entry.peer_who_told_us_handle.ban_peer(LONG_BAN));

                    return Err(e);
                }

                // Add the request to the failed list.
                if self.inflight_requests.contains_key(&start_height) {
                    self.failed_batches.push(Reverse(start_height))
                }

                Ok(())
            }
            Ok((client, block_batch)) => {
                // Remove the batch from the inflight batches.
                if self.inflight_requests.remove(&start_height).is_none() {
                    // If it was already retrieved then there is nothing else to do.
                    // TODO: should we drop this peer for being slow?
                    self.handle_free_client(chain_tracker, client).await;
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

                // Give more work to this client.
                self.handle_free_client(chain_tracker, client).await;
                Ok(())
            }
        }
    }

    async fn run(mut self) -> Result<(), BlockDownloadError> {
        let mut chain_tracker = initial_chain_search(
            &self.client_pool,
            self.peer_sync_svc.clone(),
            &mut self.our_chain_svc,
        )
        .await?;

        let mut check_client_pool_interval = interval(self.config.check_client_pool_interval);
        check_client_pool_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        self.check_for_free_clients(&mut chain_tracker).await?;

        loop {
            tokio::select! {
                _ = check_client_pool_interval.tick() => {
                    self.check_for_free_clients(&mut chain_tracker).await?;
                }
                Some(res) = self.block_download_tasks.join_next() => {
                    let (start_height, res) = res.expect("Download batch future panicked");

                    self.handle_download_batch_res(start_height, res, &mut chain_tracker).await?;
                }
                Some(Ok(res)) = self.chain_entry_task.join_next() => {
                    match res {
                        Ok((client, entry)) => {
                            if chain_tracker.add_entry(entry).is_ok() {

                            }
                            self.handle_free_client(&mut chain_tracker, client).await;
                        }
                        Err(_) => {}
                    }
                }
                else => {
                    self.check_for_free_clients(&mut chain_tracker).await?;
                }
            }
        }
    }
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

async fn request_batch_from_peer<N: NetworkZone>(
    mut client: ClientPoolDropGuard<N>,
    ids: ByteArrayVec<32>,
    expected_start_height: u64,
) -> Result<(ClientPoolDropGuard<N>, BlockBatch), BlockDownloadError> {
    let numb_requested = ids.len();

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

    if blocks_response.blocks.len() > numb_requested {
        client.info.handle.ban_peer(MEDIUM_BAN);
        return Err(BlockDownloadError::PeersResponseWasInvalid);
    }

    if blocks_response.blocks.len() != numb_requested {
        return Err(BlockDownloadError::PeerDidNotHaveRequestedData);
    }

    let blocks = rayon_spawn_async(move || {
        // TODO: size check the incoming blocks/ txs.

        let blocks = blocks_response
            .blocks
            .into_par_iter()
            .enumerate()
            .map(|(i, block)| (i, u64::try_from(i).unwrap() + expected_start_height, block))
            .map(|(i, expected_height, block_entry)| {
                let mut size = block_entry.block.len();

                let block = Block::read(&mut block_entry.block.as_ref())
                    .map_err(|_| BlockDownloadError::PeersResponseWasInvalid)?;

                if ids[i] != block.hash() || block.txs.len() != block_entry.txs.len() {
                    return Err(BlockDownloadError::PeersResponseWasInvalid);
                }

                if !block
                    .number()
                    .is_some_and(|height| height == expected_height)
                {
                    // TODO: remove this panic, I have it this error though which is why it's here.
                    panic!("{} {}", expected_height, block.number().unwrap());
                    // This peer probably did nothing wrong, it was the peer who told us this blockID which
                    // is misbehaving.
                    return Err(BlockDownloadError::ChainInvalid);
                }

                let txs = block_entry
                    .txs
                    .take_normal()
                    .ok_or(BlockDownloadError::PeersResponseWasInvalid)?
                    .into_iter()
                    .map(|tx_blob| {
                        size += tx_blob.len();
                        Transaction::read(&mut tx_blob.as_ref())
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| BlockDownloadError::PeersResponseWasInvalid)?;

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

    if chain_res.m_block_ids.is_empty() {
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

    let entry = ChainEntry {
        ids: (&chain_res.m_block_ids).into(),
        peer: client.info.id,
        handle: client.info.handle.clone(),
    };

    Ok((client, entry))
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
        futs.spawn(timeout(CHIAN_ENTRY_REQUEST_TIMEOUT, async move {
            let PeerResponse::GetChain(chain_res) =
                next_peer.ready().await?.call(cloned_req).await?
            else {
                panic!("connection task returned wrong response!");
            };

            Ok::<_, tower::BoxError>((chain_res, next_peer.info.id, next_peer.info.handle.clone()))
        }));
    }

    let mut res: Option<(ChainResponse, InternalPeerID<_>, ConnectionHandle)> = None;

    while let Some(task_res) = futs.join_next().await {
        let Ok(Ok(task_res)) = task_res.unwrap() else {
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

    let tracker = ChainTracker::new(
        first_entry,
        start_height + u64::try_from(first_unknown).unwrap(),
        our_genesis,
    );

    Ok(tracker)
}

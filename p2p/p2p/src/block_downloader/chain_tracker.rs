use cuprate_fixed_bytes::ByteArrayVec;
use std::{cmp::min, collections::VecDeque, mem};
use tower::{Service, ServiceExt};

use crate::block_downloader::{ChainSvcRequest, ChainSvcResponse};
use crate::constants::MEDIUM_BAN;
use cuprate_constants::block::MAX_BLOCK_HEIGHT_USIZE;
use cuprate_p2p_core::{client::InternalPeerID, handles::ConnectionHandle, NetworkZone};
use cuprate_pruning::PruningSeed;

/// A new chain entry to add to our chain tracker.
#[derive(Debug)]
pub struct ChainEntry<N: NetworkZone> {
    /// A list of block IDs.
    pub ids: Vec<[u8; 32]>,
    /// The peer who told us about this chain entry.
    pub peer: InternalPeerID<N::Addr>,
    /// The peer who told us about this chain entry's handle
    pub handle: ConnectionHandle,
}

/// A batch of blocks to retrieve.
#[derive(Clone)]
pub(crate) struct BlocksToRetrieve<N: NetworkZone> {
    /// The block IDs to get.
    pub ids: ByteArrayVec<32>,
    /// The hash of the last block before this batch.
    pub prev_id: [u8; 32],
    /// The expected height of the first block in [`BlocksToRetrieve::ids`].
    pub start_height: usize,
    /// The peer who told us about this batch.
    pub peer_who_told_us: InternalPeerID<N::Addr>,
    /// The peer who told us about this batch's handle.
    pub peer_who_told_us_handle: ConnectionHandle,
    /// The number of requests sent for this batch.
    pub requests_sent: usize,
    /// The number of times this batch has been requested from a peer and failed.
    pub failures: usize,
}

/// An error returned from the [`ChainTracker`].
#[derive(Debug)]
pub(crate) enum ChainTrackerError {
    /// The new chain entry is invalid.
    NewEntryIsInvalid,
    NewEntryIsEmpty,
    /// The new chain entry does not follow from the top of our chain tracker.
    NewEntryDoesNotFollowChain,

    ChainSvcError(tower::BoxError),
}

/// # Chain Tracker
///
/// This struct allows following a single chain. It takes in [`ChainEntry`]s and
/// allows getting [`BlocksToRetrieve`].
pub(crate) struct ChainTracker<N: NetworkZone> {
    /// A list of [`ChainEntry`]s, in order.
    valid_entries: VecDeque<ChainEntry<N>>,
    unknown_entries: VecDeque<ChainEntry<N>>,
    /// The height of the first block, in the first entry in [`Self::entries`].
    first_height: usize,
    /// The hash of the last block in the last entry.
    top_seen_hash: [u8; 32],
    /// The hash of the block one below [`Self::first_height`].
    previous_hash: [u8; 32],
    /// The hash of the genesis block.
    our_genesis: [u8; 32],
}

impl<N: NetworkZone> ChainTracker<N> {
    /// Creates a new chain tracker.
    pub(crate) async fn new<C>(
        new_entry: ChainEntry<N>,
        first_height: usize,
        our_genesis: [u8; 32],
        previous_hash: [u8; 32],
        our_chain_svc: &mut C,
    ) -> Result<Self, ChainTrackerError>
    where
        C: Service<ChainSvcRequest<N>, Response = ChainSvcResponse<N>, Error = tower::BoxError>,
    {
        let top_seen_hash = *new_entry.ids.last().unwrap();
        let mut entries = VecDeque::with_capacity(1);
        entries.push_back(new_entry);

        let ChainSvcResponse::ValidateEntries { valid, unknown } = our_chain_svc
            .ready()
            .await
            .map_err(ChainTrackerError::ChainSvcError)?
            .call(ChainSvcRequest::ValidateEntries(entries, first_height))
            .await
            .map_err(ChainTrackerError::ChainSvcError)?
        else {
            unreachable!()
        };

        Ok(Self {
            valid_entries: valid,
            unknown_entries: unknown,
            first_height,
            top_seen_hash,
            previous_hash,
            our_genesis,
        })
    }

    /// Returns `true` if the peer is expected to have the next block after our highest seen block
    /// according to their pruning seed.
    pub(crate) fn should_ask_for_next_chain_entry(&self, seed: &PruningSeed) -> bool {
        seed.has_full_block(self.top_height(), MAX_BLOCK_HEIGHT_USIZE)
    }

    /// Returns the simple history, the highest seen block and the genesis block.
    pub(crate) const fn get_simple_history(&self) -> [[u8; 32]; 2] {
        [self.top_seen_hash, self.our_genesis]
    }

    /// Returns the height of the highest block we are tracking.
    pub(crate) fn top_height(&self) -> usize {
        let top_block_idx = self
            .valid_entries
            .iter()
            .chain(self.unknown_entries.iter())
            .map(|entry| entry.ids.len())
            .sum::<usize>();

        self.first_height + top_block_idx
    }

    /// Returns the total number of queued batches for a certain `batch_size`.
    ///
    /// # Panics
    /// This function panics if `batch_size` is `0`.
    pub(crate) fn block_requests_queued(&self, batch_size: usize) -> usize {
        self.valid_entries
            .iter()
            .map(|entry| entry.ids.len().div_ceil(batch_size))
            .sum()
    }

    /// Attempts to add an incoming [`ChainEntry`] to the chain tracker.
    pub(crate) async fn add_entry<C>(
        &mut self,
        mut chain_entry: ChainEntry<N>,
        our_chain_svc: &mut C,
    ) -> Result<(), ChainTrackerError>
    where
        C: Service<ChainSvcRequest<N>, Response = ChainSvcResponse<N>, Error = tower::BoxError>,
    {
        if chain_entry.ids.is_empty() {
            // The peer must send at lest one overlapping block.
            chain_entry.handle.ban_peer(MEDIUM_BAN);
            return Err(ChainTrackerError::NewEntryIsInvalid);
        }

        if chain_entry.ids.len() == 1 {
            return Err(ChainTrackerError::NewEntryIsEmpty);
        }

        if self.top_seen_hash != chain_entry.ids[0] {
            return Err(ChainTrackerError::NewEntryDoesNotFollowChain);
        }

        let new_entry = ChainEntry {
            // ignore the first block - we already know it.
            ids: chain_entry.ids.split_off(1),
            peer: chain_entry.peer,
            handle: chain_entry.handle,
        };

        self.top_seen_hash = *new_entry.ids.last().unwrap();

        self.unknown_entries.push_back(new_entry);

        let ChainSvcResponse::ValidateEntries { mut valid, unknown } = our_chain_svc
            .ready()
            .await
            .map_err(ChainTrackerError::ChainSvcError)?
            .call(ChainSvcRequest::ValidateEntries(
                mem::take(&mut self.unknown_entries),
                self.first_height
                    + self
                        .valid_entries
                        .iter()
                        .map(|e| e.ids.len())
                        .sum::<usize>(),
            ))
            .await
            .map_err(ChainTrackerError::ChainSvcError)?
        else {
            unreachable!()
        };

        self.valid_entries.append(&mut valid);
        self.unknown_entries = unknown;

        Ok(())
    }

    /// Returns a batch of blocks to request.
    ///
    /// The returned batches length will be less than or equal to `max_blocks`
    pub(crate) fn blocks_to_get(
        &mut self,
        pruning_seed: &PruningSeed,
        max_blocks: usize,
    ) -> Option<BlocksToRetrieve<N>> {
        if !pruning_seed.has_full_block(self.first_height, MAX_BLOCK_HEIGHT_USIZE) {
            return None;
        }

        let entry = self.valid_entries.front_mut()?;

        // Calculate the ending index for us to get in this batch, it will be one of these:
        // - smallest out of `max_blocks`
        // - length of the batch
        // - index of the next pruned block for this seed
        let end_idx = min(
            min(entry.ids.len(), max_blocks),
                pruning_seed
                    .get_next_pruned_block(self.first_height, MAX_BLOCK_HEIGHT_USIZE)
                    .expect("We use local values to calculate height which should be below the sanity limit")
                    // Use a big value as a fallback if the seed does no pruning.
                    .unwrap_or(MAX_BLOCK_HEIGHT_USIZE)
                    - self.first_height,
        );

        if end_idx == 0 {
            return None;
        }

        let ids_to_get = entry.ids.drain(0..end_idx).collect::<Vec<_>>();

        let blocks = BlocksToRetrieve {
            ids: ids_to_get.into(),
            prev_id: self.previous_hash,
            start_height: self.first_height,
            peer_who_told_us: entry.peer,
            peer_who_told_us_handle: entry.handle.clone(),
            requests_sent: 0,
            failures: 0,
        };

        self.first_height += end_idx;
        // TODO: improve ByteArrayVec API.
        self.previous_hash = blocks.ids[blocks.ids.len() - 1];

        if entry.ids.is_empty() {
            self.valid_entries.pop_front();
        }

        Some(blocks)
    }
}

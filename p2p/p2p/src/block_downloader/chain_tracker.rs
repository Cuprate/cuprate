use std::{cmp::min, collections::VecDeque};

use cuprate_fixed_bytes::ByteArrayVec;

use cuprate_p2p_core::{client::InternalPeerID, handles::ConnectionHandle, NetworkZone};
use cuprate_pruning::{PruningSeed, CRYPTONOTE_MAX_BLOCK_HEIGHT};

use crate::constants::MEDIUM_BAN;

/// A new chain entry to add to our chain tracker.
#[derive(Debug)]
pub(crate) struct ChainEntry<N: NetworkZone> {
    /// A list of block IDs.
    pub ids: Vec<[u8; 32]>,
    /// The peer who told us about this chain entry.
    pub peer: InternalPeerID<N::Addr>,
    /// The peer who told us about this chain entry's handle
    pub handle: ConnectionHandle,
}

/// A batch of blocks to retrieve.
#[derive(Clone)]
pub struct BlocksToRetrieve<N: NetworkZone> {
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
#[derive(Debug, Clone)]
pub enum ChainTrackerError {
    /// The new chain entry is invalid.
    NewEntryIsInvalid,
    /// The new chain entry does not follow from the top of our chain tracker.
    NewEntryDoesNotFollowChain,
}

/// # Chain Tracker
///
/// This struct allows following a single chain. It takes in [`ChainEntry`]s and
/// allows getting [`BlocksToRetrieve`].
pub struct ChainTracker<N: NetworkZone> {
    /// A list of [`ChainEntry`]s, in order.
    entries: VecDeque<ChainEntry<N>>,
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
    pub fn new(
        new_entry: ChainEntry<N>,
        first_height: usize,
        our_genesis: [u8; 32],
        previous_hash: [u8; 32],
    ) -> Self {
        let top_seen_hash = *new_entry.ids.last().unwrap();
        let mut entries = VecDeque::with_capacity(1);
        entries.push_back(new_entry);

        Self {
            top_seen_hash,
            entries,
            first_height,
            previous_hash,
            our_genesis,
        }
    }

    /// Returns `true` if the peer is expected to have the next block after our highest seen block
    /// according to their pruning seed.
    pub fn should_ask_for_next_chain_entry(&self, seed: &PruningSeed) -> bool {
        seed.has_full_block(self.top_height(), CRYPTONOTE_MAX_BLOCK_HEIGHT)
    }

    /// Returns the simple history, the highest seen block and the genesis block.
    pub fn get_simple_history(&self) -> [[u8; 32]; 2] {
        [self.top_seen_hash, self.our_genesis]
    }

    /// Returns the height of the highest block we are tracking.
    pub fn top_height(&self) -> usize {
        let top_block_idx = self
            .entries
            .iter()
            .map(|entry| entry.ids.len())
            .sum::<usize>();

        self.first_height + top_block_idx
    }

    /// Returns the total number of queued batches for a certain `batch_size`.
    ///
    /// # Panics
    /// This function panics if `batch_size` is `0`.
    pub fn block_requests_queued(&self, batch_size: usize) -> usize {
        self.entries
            .iter()
            .map(|entry| entry.ids.len().div_ceil(batch_size))
            .sum()
    }

    /// Attempts to add an incoming [`ChainEntry`] to the chain tracker.
    pub fn add_entry(&mut self, mut chain_entry: ChainEntry<N>) -> Result<(), ChainTrackerError> {
        if chain_entry.ids.is_empty() {
            // The peer must send at lest one overlapping block.
            chain_entry.handle.ban_peer(MEDIUM_BAN);
            return Err(ChainTrackerError::NewEntryIsInvalid);
        }

        if chain_entry.ids.len() == 1 {
            return Err(ChainTrackerError::NewEntryDoesNotFollowChain);
        }

        if self
            .entries
            .back()
            .is_some_and(|last_entry| last_entry.ids.last().unwrap() != &chain_entry.ids[0])
        {
            return Err(ChainTrackerError::NewEntryDoesNotFollowChain);
        }

        let new_entry = ChainEntry {
            // ignore the first block - we already know it.
            ids: chain_entry.ids.split_off(1),
            peer: chain_entry.peer,
            handle: chain_entry.handle,
        };

        self.top_seen_hash = *new_entry.ids.last().unwrap();

        self.entries.push_back(new_entry);

        Ok(())
    }

    /// Returns a batch of blocks to request.
    ///
    /// The returned batches length will be less than or equal to `max_blocks`
    pub fn blocks_to_get(
        &mut self,
        pruning_seed: &PruningSeed,
        max_blocks: usize,
    ) -> Option<BlocksToRetrieve<N>> {
        if !pruning_seed.has_full_block(self.first_height, CRYPTONOTE_MAX_BLOCK_HEIGHT) {
            return None;
        }

        let entry = self.entries.front_mut()?;

        // Calculate the ending index for us to get in this batch, it will be one of these:
        // - smallest out of `max_blocks`
        // - length of the batch
        // - index of the next pruned block for this seed
        let end_idx = min(
            min(entry.ids.len(), max_blocks),
                pruning_seed
                    .get_next_pruned_block(self.first_height, CRYPTONOTE_MAX_BLOCK_HEIGHT)
                    .expect("We use local values to calculate height which should be below the sanity limit")
                    // Use a big value as a fallback if the seed does no pruning.
                    .unwrap_or(CRYPTONOTE_MAX_BLOCK_HEIGHT)
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
            self.entries.pop_front();
        }

        Some(blocks)
    }
}

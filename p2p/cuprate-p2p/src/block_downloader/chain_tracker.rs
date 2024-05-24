use fixed_bytes::ByteArrayVec;
use std::{cmp::min, collections::VecDeque};

use monero_p2p::{client::InternalPeerID, handles::ConnectionHandle, NetworkZone};
use monero_pruning::{PruningSeed, CRYPTONOTE_MAX_BLOCK_HEIGHT};
use monero_wire::protocol::ChainResponse;

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
pub struct BlocksToRetrieve<N: NetworkZone> {
    /// The block IDs to get.
    pub ids: ByteArrayVec<32>,
    /// The expected height of the first block in `ids`.
    pub start_height: u64,
    /// The peer who told us about this batch.
    pub peer_who_told_us: InternalPeerID<N::Addr>,
    /// The peer who told us about this batch's handle.
    pub peer_who_told_us_handle: ConnectionHandle,
}

pub enum ChainTrackerError {
    NewEntryIsInvalid,
    NewEntryDoesNotFollowChain,
}

/// # Chain Tracker
///
/// This struct allows following a single chain. It takes in [`ChainEntry`]s and
/// allows getting [`BlocksToRetrieve`].
pub struct ChainTracker<N: NetworkZone> {
    /// A list of [`ChainEntry`]s, in order.
    entries: VecDeque<ChainEntry<N>>,
    /// The height of the first block, in the first entry in entries.
    first_height: u64,
    /// The hash of the last block in the last entry.
    top_seen_hash: [u8; 32],
    /// The hash of the genesis block.
    our_genesis: [u8; 32],
}

impl<N: NetworkZone> ChainTracker<N> {
    pub fn new(new_entry: ChainEntry<N>, first_height: u64, our_genesis: [u8; 32]) -> Self {
        let top_seen_hash = *new_entry.ids.last().unwrap();
        let mut entries = VecDeque::with_capacity(1);
        entries.push_back(new_entry);

        Self {
            top_seen_hash,
            entries,
            first_height,
            our_genesis,
        }
    }

    /// Returns `true` if the peer is expected to have the next block after our highest seen block
    /// according to their pruning seed.
    pub fn should_ask_for_next_chain_entry(&self, seed: &PruningSeed) -> bool {
        let top_block_idx = self
            .entries
            .iter()
            .map(|entry| entry.ids.len())
            .sum::<usize>();

        seed.has_full_block(
            self.first_height + u64::try_from(top_block_idx).unwrap(),
            CRYPTONOTE_MAX_BLOCK_HEIGHT,
        )
    }

    /// Returns the simple history, the highest seen block and the genesis block.
    pub fn get_simple_history(&self) -> [[u8; 32]; 2] {
        [self.top_seen_hash, self.our_genesis]
    }

    /// Returns the total number of queued batches for a certain `batch_size`.
    pub fn block_requests_queued(&self, batch_size: usize) -> usize {
        self.entries
            .iter()
            .map(|entry| entry.ids.len().div_ceil(batch_size))
            .sum()
    }

    pub fn add_entry(
        &mut self,
        mut chain_entry: ChainResponse,
        peer: InternalPeerID<N::Addr>,
        handle: ConnectionHandle,
    ) -> Result<(), ChainTrackerError> {
        // TODO: check chain entries length.
        if chain_entry.m_block_ids.is_empty() {
            // The peer must send at lest one overlapping block.
            handle.ban_peer(MEDIUM_BAN);
            return Err(ChainTrackerError::NewEntryIsInvalid);
        }

        if self
            .entries
            .back()
            .is_some_and(|last_entry| last_entry.ids.last().unwrap() != &chain_entry.m_block_ids[0])
        {
            return Err(ChainTrackerError::NewEntryDoesNotFollowChain);
        }

        tracing::warn!("len: {}", chain_entry.m_block_ids.len());

        let new_entry = ChainEntry {
            // ignore the first block - we already know it.
            ids: (&chain_entry.m_block_ids.split_off(1)).into(),
            peer,
            handle,
        };

        self.top_seen_hash = *new_entry.ids.last().unwrap();

        self.entries.push_back(new_entry);

        Ok(())
    }

    pub fn blocks_to_get(
        &mut self,
        pruning_seed: &PruningSeed,
        max_blocks: usize,
    ) -> Option<BlocksToRetrieve<N>> {
        if !pruning_seed.has_full_block(self.first_height, CRYPTONOTE_MAX_BLOCK_HEIGHT) {
            return None;
        }

        // TODO: make sure max block height is enforced.

        let entry = self.entries.front_mut()?;

        // Calculate the ending index for us to get in this batch, will be the smallest out of `max_blocks`, the length of the batch or
        // the index of the next pruned block for this seed.
        let end_idx = min(
            min(entry.ids.len(), max_blocks),
            usize::try_from(
                pruning_seed
                    .get_next_pruned_block(self.first_height, CRYPTONOTE_MAX_BLOCK_HEIGHT)
                    // We check the first height is less than CRYPTONOTE_MAX_BLOCK_HEIGHT in response task.
                    .unwrap()
                    // Use a big value as a fallback if the seed does no pruning.
                    .unwrap_or(CRYPTONOTE_MAX_BLOCK_HEIGHT)
                    - self.first_height,
            )
            .unwrap(),
        );

        if end_idx == 0 {
            return None;
        }

        let ids_to_get = entry.ids.drain(0..end_idx).collect::<Vec<_>>();

        let blocks = BlocksToRetrieve {
            ids: ids_to_get.into(),
            start_height: self.first_height,
            peer_who_told_us: entry.peer,
            peer_who_told_us_handle: entry.handle.clone(),
        };

        self.first_height += u64::try_from(end_idx).unwrap();

        if entry.ids.is_empty() {
            self.entries.pop_front();
        }

        Some(blocks)
    }
}

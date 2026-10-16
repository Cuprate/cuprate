//! Tracking of the blocks sent to the blockchain manager.
use std::{
    collections::{hash_map::Entry, HashMap},
    hash::{BuildHasher, RandomState},
    sync::{Arc, OnceLock, RwLock, Weak},
};

use bytes::Bytes;

use cuprate_consensus_rules::{blocks::BlockError, transactions::TransactionError, ConsensusError};
use cuprate_types::TransactionVerificationData;

use crate::{
    blockchain::{manager::IncomingBlockOk, IncomingBlockError},
    constants::MAX_INCOMING_BLOCK_DEPTH,
};

/// The state of a block sent to the blockchain manager.
#[derive(Clone, Copy)]
enum BlockState {
    /// The blockchain manager is handling it.
    BeingHandled,
    /// It was added to the main chain.
    Added,
    /// It failed verification with valid proof-of-work, for this reason.
    Invalid(ConsensusError),
    /// Nothing about it needs to be remembered.
    Forgotten,
}

impl BlockState {
    /// Returns the response for a block that does not need to be sent to the blockchain manager.
    const fn response(self) -> Option<Result<IncomingBlockOk, IncomingBlockError>> {
        match self {
            Self::BeingHandled => Some(Ok(IncomingBlockOk::AlreadyHave)),
            Self::Invalid(inner) => Some(Err(IncomingBlockError::Validation {
                pow_valid: true,
                inner,
            })),
            Self::Added | Self::Forgotten => None,
        }
    }
}

/// What the blockchain manager shares about a block it was sent.
#[derive(Default)]
struct Outcome {
    /// The state the blockchain manager left the block in, unset while it is handling the block.
    state: OnceLock<BlockState>,
    /// The block, while it is being handled after being announced to peers.
    announced: OnceLock<Weak<AnnouncedBlock>>,
}

/// A block announced to peers before it is in the database, kept to send them its txs.
pub(crate) struct AnnouncedBlock {
    /// The serialized block.
    pub(crate) block_blob: Bytes,
    /// The serialized txs, by hash.
    pub(crate) txs: HashMap<[u8; 32], Bytes>,
}

/// The blockchain manager's handle to a block's [`Outcome`].
#[derive(Default)]
pub(crate) struct BlockOutcome {
    cell: Arc<Outcome>,
    /// The block, if it was announced to peers while being handled.
    announced: Option<Arc<AnnouncedBlock>>,
}

impl BlockOutcome {
    /// Marks the block as announced to peers, so any failure is remembered and it is not announced again.
    ///
    /// Until the block is handled, its txs are sent to peers from here.
    pub(crate) fn set_announced(
        &mut self,
        block_blob: Bytes,
        txs: &HashMap<[u8; 32], TransactionVerificationData>,
    ) {
        let block = self.announced.insert(Arc::new(AnnouncedBlock {
            block_blob,
            txs: txs
                .iter()
                .map(|(hash, tx)| (*hash, Bytes::copy_from_slice(&tx.tx_blob)))
                .collect(),
        }));
        let _ = self.cell.announced.set(Arc::downgrade(block));
    }

    /// Record what the blockchain manager's response says about the block.
    pub(crate) fn record(&self, res: &Result<IncomingBlockOk, IncomingBlockError>) {
        let state = match res {
            Ok(IncomingBlockOk::AddedToMainChain) => BlockState::Added,
            Err(IncomingBlockError::Validation {
                pow_valid: true,
                inner,
            }) if self.announced.is_some() => BlockState::Invalid(*inner),
            Err(IncomingBlockError::Validation {
                pow_valid: true,
                inner,
            }) => match inner {
                // These can pass later, or are not about the block itself.
                ConsensusError::Block(
                    BlockError::TimeStampInFuture
                    | BlockError::PreviousIDIncorrect
                    | BlockError::TxsIncludedWithBlockIncorrect,
                )
                | ConsensusError::Transaction(TransactionError::OneOrMoreRingMembersLocked) => {
                    BlockState::Forgotten
                }
                ConsensusError::Block(_) | ConsensusError::Transaction(_) => {
                    BlockState::Invalid(*inner)
                }
            },
            _ => BlockState::Forgotten,
        };
        let _ = self.cell.state.set(state);
    }
}

impl Drop for BlockOutcome {
    fn drop(&mut self) {
        let _ = self.cell.state.set(BlockState::Forgotten);
    }
}

/// A block sent to the blockchain manager.
struct KnownBlock {
    outcome: Arc<Outcome>,
    height: usize,
    /// The length of the serialized block.
    blob_len: usize,
    /// The [`KnownBlocks::blob_hash`] of the block.
    blob_hash: u64,
}

impl KnownBlock {
    /// Returns the block's state.
    fn state(&self) -> BlockState {
        self.outcome
            .state
            .get()
            .copied()
            .unwrap_or(BlockState::BeingHandled)
    }
}

/// The blocks that the blockchain manager is currently handling, added or found to be invalid, by hash.
///
/// This prevents sending the same block to the blockchain manager from multiple connections
/// before one of them actually gets added to the chain, allowing peers to do other things,
/// and verifying an invalid block more than once.
///
/// This is used over something like a dashmap as we expect a lot of collisions in a short amount of
/// time for new blocks, so we would lose the benefit of sharded locks.
#[derive(Clone, Default)]
pub(crate) struct KnownBlocks {
    blocks: Arc<RwLock<HashMap<[u8; 32], KnownBlock>>>,
    /// Hashes serialized blocks with a random key, so that a peer cannot make two look the same.
    blob_hasher: RandomState,
}

impl KnownBlocks {
    /// Returns the [`BlockState`] of the block with the given hash, if it is known.
    fn state(&self, hash: &[u8; 32]) -> Option<BlockState> {
        self.blocks.read().unwrap().get(hash).map(KnownBlock::state)
    }

    /// Returns `true` if the given block hash is currently being handled.
    pub(crate) fn is_being_handled(&self, hash: &[u8; 32]) -> bool {
        matches!(self.state(hash), Some(BlockState::BeingHandled))
    }

    /// Returns the block with the given hash, while it is being handled after being announced to peers.
    pub(crate) fn announced_block(&self, hash: &[u8; 32]) -> Option<Arc<AnnouncedBlock>> {
        self.blocks
            .read()
            .unwrap()
            .get(hash)?
            .outcome
            .announced
            .get()?
            .upgrade()
    }

    /// Returns `true` if the block was sent to the blockchain manager.
    pub(crate) fn contains(&self, hash: &[u8; 32]) -> bool {
        self.state(hash).is_some()
    }

    /// Returns a hash of the serialized block.
    fn blob_hash(&self, block_blob: &[u8]) -> u64 {
        self.blob_hasher.hash_one(block_blob)
    }

    /// Returns `true` if the serialized block is currently being handled, is our top block or is
    /// known to be invalid.
    pub(crate) fn is_blob_known(&self, block_blob: &[u8], top_hash: &[u8; 32]) -> bool {
        let mut blob_hash = None;
        self.blocks.read().unwrap().iter().any(|(hash, known)| {
            known.blob_len == block_blob.len()
                && (hash == top_hash
                    || matches!(
                        known.state(),
                        BlockState::BeingHandled | BlockState::Invalid(_)
                    ))
                // Only a block with the length of a known one is hashed.
                && known.blob_hash == *blob_hash.get_or_insert_with(|| self.blob_hash(block_blob))
        })
    }

    /// Returns the response for a block that does not need to be sent to the blockchain manager.
    pub(super) fn response(
        &self,
        hash: &[u8; 32],
    ) -> Option<Result<IncomingBlockOk, IncomingBlockError>> {
        self.state(hash)?.response()
    }

    /// Adds a block as being handled, returning where the blockchain manager records its outcome.
    ///
    /// Returns the block's [`Self::response`] if it is already being handled or is known to be invalid.
    pub(super) fn insert(
        &self,
        block_hash: [u8; 32],
        height: usize,
        block_blob: &[u8],
    ) -> Result<BlockOutcome, Result<IncomingBlockOk, IncomingBlockError>> {
        let blob_hash = self.blob_hash(block_blob);
        let mut blocks = self.blocks.write().unwrap();
        // Remove the blocks the manager is done with.
        blocks.retain(|_, known| match known.state() {
            BlockState::BeingHandled => true,
            // An added block is only known while it is our top block.
            BlockState::Added => known.height + 1 >= height,
            BlockState::Invalid(_) => known.height + MAX_INCOMING_BLOCK_DEPTH >= height,
            BlockState::Forgotten => false,
        });

        let entry = blocks.entry(block_hash);
        if let Entry::Occupied(known) = &entry {
            if let Some(res) = known.get().state().response() {
                return Err(res);
            }
        }

        let outcome = BlockOutcome::default();
        entry.insert_entry(KnownBlock {
            outcome: Arc::clone(&outcome.cell),
            height,
            blob_len: block_blob.len(),
            blob_hash,
        });
        drop(blocks);

        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn announced_block_is_only_kept_while_handled() {
        let known_blocks = KnownBlocks::default();
        let mut outcome = known_blocks.insert([1; 32], 1, &[1]).ok().unwrap();
        assert!(known_blocks.announced_block(&[1; 32]).is_none());

        outcome.set_announced(Bytes::from_static(&[1]), &HashMap::new());
        assert_eq!(
            known_blocks.announced_block(&[1; 32]).unwrap().block_blob,
            Bytes::from_static(&[1])
        );

        drop(outcome);
        assert!(known_blocks.announced_block(&[1; 32]).is_none());
    }
}

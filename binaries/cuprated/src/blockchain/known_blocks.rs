//! Tracking of the blocks sent to the blockchain manager.
use std::{
    collections::{hash_map::Entry, HashMap},
    hash::{BuildHasher, RandomState},
    sync::{Arc, OnceLock, RwLock},
};

use monero_oxide::block::Block;

use cuprate_consensus_rules::{blocks::BlockError, ConsensusError};

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

/// The blockchain manager's handle to the [`BlockState`] it leaves a block in.
///
/// If it is dropped before [`Self::record`], the block is forgotten so it can be sent again.
#[derive(Default)]
pub(crate) struct BlockOutcome(Arc<OnceLock<BlockState>>);

impl BlockOutcome {
    /// Record what the blockchain manager's response says about the block.
    pub(super) fn record(self, res: &Result<IncomingBlockOk, IncomingBlockError>) {
        let state = match res {
            Ok(IncomingBlockOk::AddedToMainChain) => BlockState::Added,
            Err(IncomingBlockError::Validation {
                pow_valid: true,
                inner,
            }) => match inner {
                // These can pass later, or are not about the block itself.
                ConsensusError::Block(
                    BlockError::TimeStampInFuture
                    | BlockError::PreviousIDIncorrect
                    | BlockError::TxsIncludedWithBlockIncorrect,
                ) => BlockState::Forgotten,
                ConsensusError::Block(_) | ConsensusError::Transaction(_) => {
                    BlockState::Invalid(*inner)
                }
            },
            _ => BlockState::Forgotten,
        };
        let _ = self.0.set(state);
    }
}

impl Drop for BlockOutcome {
    fn drop(&mut self) {
        let _ = self.0.set(BlockState::Forgotten);
    }
}

/// A block sent to the blockchain manager.
struct KnownBlock {
    /// The block's state, set when its [`BlockOutcome`] is recorded or dropped.
    outcome: Arc<OnceLock<BlockState>>,
    /// The block's height.
    height: usize,
    /// The length of the serialized block.
    blob_len: usize,
    /// The hash of the serialized block, from [`KnownBlocks::blob_hasher`].
    blob_hash: u64,
}

impl KnownBlock {
    /// Returns the block's state.
    fn state(&self) -> BlockState {
        self.outcome
            .get()
            .copied()
            .unwrap_or(BlockState::BeingHandled)
    }
}

/// The blocks that the blockchain manager is currently handling, added or found to be invalid, by hash.
///
/// This prevents sending the same block to the blockchain manager from multiple connections
/// before one of them actually gets added to the chain, allowing peers to do other things.
/// It also prevents verifying an invalid block more than once.
///
/// This is used over something like a dashmap as we expect a lot of collisions in a short amount of
/// time for new blocks, so we would lose the benefit of sharded locks. It is an [`RwLock`] as only
/// [`Self::insert`] takes the write lock, so peers can look blocks up at the same time.
#[derive(Clone, Default)]
pub(crate) struct KnownBlocks {
    /// The known blocks, by block hash.
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
    pub(super) fn is_being_handled(&self, hash: &[u8; 32]) -> bool {
        matches!(self.state(hash), Some(BlockState::BeingHandled))
    }

    /// Returns `true` if the serialized block is currently being handled, is known to be invalid, or
    /// is our top block and was sent to the blockchain manager.
    pub(crate) fn is_blob_known(&self, block_blob: &[u8], top_hash: &[u8; 32]) -> bool {
        // Only a block with the length of a known one is hashed.
        let mut blob_hash = None;
        self.blocks.read().unwrap().iter().any(|(hash, known)| {
            known.blob_len == block_blob.len()
                && (hash == top_hash || known.state().response().is_some())
                && known.blob_hash
                    == *blob_hash.get_or_insert_with(|| self.blob_hasher.hash_one(block_blob))
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
        block: &Block,
    ) -> Result<BlockOutcome, Result<IncomingBlockOk, IncomingBlockError>> {
        let block_blob = block.serialize();
        let blob_hash = self.blob_hasher.hash_one(&block_blob);
        let mut blocks = self.blocks.write().unwrap();
        // Remove the blocks that no longer need to be remembered.
        blocks.retain(|_, known| match known.state() {
            BlockState::BeingHandled => true,
            // An added block is kept while it may still be our top block, see `is_blob_known`.
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
            outcome: Arc::clone(&outcome.0),
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
    use monero_oxide::{
        block::BlockHeader,
        transaction::{Input, Timelock, Transaction, TransactionPrefix},
    };

    use super::*;

    /// Tests that [`KnownBlocks`] keeps, forgets and evicts blocks by their outcome.
    #[test]
    fn outcomes() {
        let header = BlockHeader {
            hardfork_version: 16,
            hardfork_signal: 16,
            timestamp: 0,
            previous: [0; 32],
            nonce: 0,
        };
        let prefix = TransactionPrefix {
            additional_timelock: Timelock::None,
            inputs: vec![Input::Gen(0)],
            outputs: vec![],
            extra: vec![],
        };
        let miner_tx = Transaction::V2 {
            prefix,
            proofs: None,
        };
        let block = Block::new(header, miner_tx, vec![]).unwrap();
        let blob = block.serialize();
        let mut other_blob = blob.clone();
        other_blob[0] ^= 1;
        let (hash, top_hash) = ([1; 32], [2; 32]);
        let invalid = |e| {
            Err(IncomingBlockError::Validation {
                pow_valid: true,
                inner: ConsensusError::Block(e),
            })
        };
        let known_blocks = KnownBlocks::default();
        let insert = |hash, height| known_blocks.insert(hash, height, &block).ok().unwrap();

        // A block is being handled until its outcome is dropped.
        let outcome = insert(hash, 1);
        assert!(known_blocks.is_being_handled(&hash));
        assert!(known_blocks.is_blob_known(&blob, &top_hash));
        assert!(!known_blocks.is_blob_known(&other_blob, &top_hash));
        assert!(matches!(
            known_blocks.insert(hash, 1, &block),
            Err(Ok(IncomingBlockOk::AlreadyHave))
        ));
        drop(outcome);
        assert!(known_blocks.response(&hash).is_none());

        // A failure that can pass later is forgotten.
        insert(hash, 1).record(&invalid(BlockError::TimeStampInFuture));
        assert!(known_blocks.response(&hash).is_none());

        // An added block is only known while it is our top block.
        insert(hash, 1).record(&Ok(IncomingBlockOk::AddedToMainChain));
        assert!(known_blocks.is_blob_known(&blob, &hash));
        assert!(!known_blocks.is_blob_known(&blob, &top_hash));
        assert!(known_blocks.response(&hash).is_none());

        // An invalid block is refused until it is too deep.
        insert(hash, 1).record(&invalid(BlockError::TimeStampInvalid));
        assert!(known_blocks.is_blob_known(&blob, &top_hash));
        drop(insert(top_hash, 1 + MAX_INCOMING_BLOCK_DEPTH));
        assert!(matches!(known_blocks.response(&hash), Some(Err(_))));
        drop(insert(top_hash, 2 + MAX_INCOMING_BLOCK_DEPTH));
        assert!(known_blocks.response(&hash).is_none());
    }
}

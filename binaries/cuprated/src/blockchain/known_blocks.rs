//! Tracking of the blocks sent to the blockchain manager.
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::{Arc, Mutex},
};

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
    /// We have it.
    Added,
    /// It failed verification with valid proof-of-work, for this reason.
    Invalid(ConsensusError),
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
            Self::Added => None,
        }
    }
}

/// The blockchain manager's handle to a block it is handling.
///
/// When dropped, it leaves the block in the state set by [`Self::record`], or forgets the block so it can
/// be sent again.
#[cfg_attr(test, derive(Default))]
pub(crate) struct BlockOutcome {
    /// The known blocks.
    known_blocks: KnownBlocks,
    /// The block's hash.
    hash: [u8; 32],
    /// The state to leave the block in, [`None`] to forget it.
    state: Option<BlockState>,
}

impl BlockOutcome {
    /// Record what the blockchain manager's response says about the block.
    ///
    /// A failure is only remembered if the block extended our top block, as only then was its
    /// proof-of-work checked against our chain's difficulty.
    pub(super) fn record(
        mut self,
        res: &Result<IncomingBlockOk, IncomingBlockError>,
        extends_top: bool,
    ) {
        self.state = match res {
            Ok(IncomingBlockOk::AddedToMainChain | IncomingBlockOk::AlreadyHave) => {
                Some(BlockState::Added)
            }
            Err(IncomingBlockError::Validation {
                pow_valid: true,
                inner,
            }) if extends_top => match inner {
                // These can pass later, or are not about the block itself.
                ConsensusError::Block(
                    BlockError::TimeStampInFuture
                    | BlockError::PreviousIDIncorrect
                    | BlockError::TxsIncludedWithBlockIncorrect,
                ) => None,
                ConsensusError::Block(_) | ConsensusError::Transaction(_) => {
                    Some(BlockState::Invalid(*inner))
                }
            },
            _ => None,
        };
    }
}

impl Drop for BlockOutcome {
    fn drop(&mut self) {
        let mut blocks = self.known_blocks.blocks.lock().unwrap();
        if let Some(state) = self.state {
            if let Some(known) = blocks.get_mut(&self.hash) {
                if let BlockState::Invalid(_) = state {
                    known.blob = Box::default();
                }
                known.state = state;
            }
        } else {
            let known = blocks.remove(&self.hash);
            drop(blocks);
            drop(known);
        }
    }
}

/// A block sent to the blockchain manager.
struct KnownBlock {
    /// The block's state.
    state: BlockState,
    /// The block's height.
    height: usize,
    /// The serialized block, empty once it is known to be invalid.
    blob: Box<[u8]>,
}

/// The blocks that the blockchain manager is currently handling, added or found to be invalid, by hash.
///
/// This prevents sending the same block to the blockchain manager from multiple connections
/// before one of them actually gets added to the chain, allowing peers to do other things.
/// It also prevents verifying an invalid block more than once.
///
/// This is used over something like a dashmap as we expect a lot of collisions in a short amount of
/// time for new blocks, so we would lose the benefit of sharded locks. A dashmap is made up of `RwLocks`
/// which are also more expensive than `Mutex`s.
#[derive(Clone, Default)]
pub(crate) struct KnownBlocks {
    /// The known blocks, by block hash.
    blocks: Arc<Mutex<HashMap<[u8; 32], KnownBlock>>>,
}

impl KnownBlocks {
    /// Returns the [`BlockState`] of the block with the given hash, if it is known.
    fn state(&self, hash: &[u8; 32]) -> Option<BlockState> {
        self.blocks
            .lock()
            .unwrap()
            .get(hash)
            .map(|known| known.state)
    }

    /// Returns `true` if the given block hash is currently being handled.
    pub(super) fn is_being_handled(&self, hash: &[u8; 32]) -> bool {
        matches!(self.state(hash), Some(BlockState::BeingHandled))
    }

    /// Returns `true` only if the serialized block is currently being handled, or is our top block
    /// and was sent to the blockchain manager.
    pub(crate) fn is_blob_known(&self, block_blob: &[u8], top_hash: &[u8; 32]) -> bool {
        let mut first_being_handled = true;

        self.blocks
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, known)| known.blob.len() == block_blob.len())
            .any(|(hash, known)| match known.state {
                BlockState::BeingHandled if first_being_handled => {
                    first_being_handled = false;
                    *known.blob == *block_blob
                }
                BlockState::Added if hash == top_hash => *known.blob == *block_blob,
                BlockState::BeingHandled | BlockState::Added | BlockState::Invalid(_) => false,
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
        blob: Box<[u8]>,
    ) -> Result<BlockOutcome, Result<IncomingBlockOk, IncomingBlockError>> {
        let known = KnownBlock {
            state: BlockState::BeingHandled,
            height,
            blob,
        };

        let mut blocks = self.blocks.lock().unwrap();
        // Remove the blocks that no longer need to be remembered.
        blocks.retain(|_, known| match known.state {
            BlockState::BeingHandled => true,
            // An added block is kept while it may still be our top block, see `is_blob_known`.
            BlockState::Added => known.height + 1 >= height,
            BlockState::Invalid(_) => known.height + MAX_INCOMING_BLOCK_DEPTH >= height,
        });

        match blocks.entry(block_hash) {
            Entry::Occupied(mut entry) => {
                if let Some(res) = entry.get().state.response() {
                    return Err(res);
                }

                entry.insert(known);
            }
            Entry::Vacant(entry) => {
                entry.insert(known);
            }
        }
        drop(blocks);

        Ok(BlockOutcome {
            known_blocks: self.clone(),
            hash: block_hash,
            state: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that [`KnownBlocks`] keeps, forgets and evicts blocks by their outcome.
    #[test]
    fn outcomes() {
        let blob: Box<[u8]> = Box::new([0; 8]);
        let (hash, top_hash) = ([1; 32], [2; 32]);
        let invalid = |e| {
            Err(IncomingBlockError::Validation {
                pow_valid: true,
                inner: ConsensusError::Block(e),
            })
        };
        let known_blocks = KnownBlocks::default();
        let insert = |hash, height| {
            known_blocks
                .insert(hash, height, blob.clone())
                .ok()
                .unwrap()
        };

        // A block is being handled until its outcome is dropped.
        let outcome = insert(hash, 1);
        assert!(known_blocks.is_being_handled(&hash));
        assert!(known_blocks.is_blob_known(&blob, &top_hash));
        assert!(!known_blocks.is_blob_known(&[1; 8], &top_hash));
        assert!(matches!(
            known_blocks.insert(hash, 1, blob.clone()),
            Err(Ok(IncomingBlockOk::AlreadyHave))
        ));
        drop(outcome);
        assert!(known_blocks.response(&hash).is_none());
        assert!(!known_blocks.is_blob_known(&blob, &hash));

        // A failure that can pass later is forgotten.
        insert(hash, 1).record(&invalid(BlockError::TimeStampInFuture), true);
        assert!(known_blocks.response(&hash).is_none());

        // An added block is only known while it is our top block.
        insert(hash, 1).record(&Ok(IncomingBlockOk::AddedToMainChain), true);
        assert!(known_blocks.is_blob_known(&blob, &hash));
        assert!(!known_blocks.is_blob_known(&blob, &top_hash));
        assert!(known_blocks.response(&hash).is_none());

        // An invalid block that did not extend our top block is forgotten.
        insert(hash, 1).record(&invalid(BlockError::TimeStampInvalid), false);
        assert!(known_blocks.response(&hash).is_none());

        // An invalid block is refused until it is too deep, and its blob is not kept.
        insert(hash, 1).record(&invalid(BlockError::TimeStampInvalid), true);
        assert!(known_blocks.blocks.lock().unwrap()[&hash].blob.is_empty());
        assert!(!known_blocks.is_blob_known(&[], &hash));
        drop(insert(top_hash, 1 + MAX_INCOMING_BLOCK_DEPTH));
        assert!(matches!(known_blocks.response(&hash), Some(Err(_))));
        drop(insert(top_hash, 2 + MAX_INCOMING_BLOCK_DEPTH));
        assert!(known_blocks.response(&hash).is_none());
    }
}

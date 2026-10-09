//! Tracking of the blocks sent to the blockchain manager.
use std::{
    collections::{hash_map::Entry, HashMap},
    hash::{BuildHasher, RandomState},
    sync::{Arc, RwLock},
};

use tokio::sync::SetOnce;

use cuprate_consensus_rules::{blocks::BlockError, transactions::TransactionError, ConsensusError};

use crate::{
    blockchain::{manager::IncomingBlockOk, IncomingBlockError},
    constants::MAX_INCOMING_BLOCK_DEPTH,
};

/// The state of a block sent to the blockchain manager.
#[derive(Clone, Copy)]
enum BlockState {
    /// The blockchain manager is handling it.
    BeingHandled,
    /// It was announced to peers, with nothing else to remember.
    Announced,
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
            Self::Announced | Self::Added | Self::Forgotten => None,
        }
    }
}

/// The state the blockchain manager left a block in, unset while it is handling the block.
type Outcome = Arc<SetOnce<BlockState>>;

/// The blockchain manager's handle to a block's [`Outcome`].
#[derive(Default)]
pub(crate) struct BlockOutcome {
    cell: Outcome,
    /// If the block was announced to peers.
    announced: bool,
}

impl BlockOutcome {
    /// Returns `true` if the block is to be announced to peers, and marks it as announced.
    pub(crate) const fn should_announce(&mut self) -> bool {
        !std::mem::replace(&mut self.announced, true)
    }

    /// Record what the blockchain manager's response says about the block.
    ///
    /// A response with nothing to remember is left to this type's [`Drop`].
    pub(crate) fn record(self, res: &Result<IncomingBlockOk, IncomingBlockError>) {
        let state = match res {
            Ok(IncomingBlockOk::AddedToMainChain) => BlockState::Added,
            Err(IncomingBlockError::Validation {
                pow_valid: true,
                inner,
            }) => match inner {
                // These can pass later, or are not about the block itself.
                ConsensusError::Block(
                    BlockError::TimeStampInvalid
                    | BlockError::PreviousIDIncorrect
                    | BlockError::TxsIncludedWithBlockIncorrect,
                )
                | ConsensusError::Transaction(TransactionError::OneOrMoreRingMembersLocked) => {
                    return;
                }
                ConsensusError::Block(_) | ConsensusError::Transaction(_) => {
                    BlockState::Invalid(*inner)
                }
            },
            _ => return,
        };
        let _ = self.cell.set(state);
    }
}

impl Drop for BlockOutcome {
    fn drop(&mut self) {
        let _ = self.cell.set(if self.announced {
            BlockState::Announced
        } else {
            BlockState::Forgotten
        });
    }
}

/// A block sent to the blockchain manager.
struct KnownBlock {
    outcome: Outcome,
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

    /// Waits for the blockchain manager to be done with the block, if it is being handled.
    pub(crate) async fn wait_until_handled(&self, hash: &[u8; 32]) {
        let outcome = self
            .blocks
            .read()
            .unwrap()
            .get(hash)
            .map(|known| Arc::clone(&known.outcome));
        if let Some(outcome) = outcome {
            outcome.wait().await;
        }
    }

    /// Returns `true` if the given block hash is currently being handled.
    pub(crate) fn is_being_handled(&self, hash: &[u8; 32]) -> bool {
        matches!(self.state(hash), Some(BlockState::BeingHandled))
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
            BlockState::Announced | BlockState::Invalid(_) => {
                known.height + MAX_INCOMING_BLOCK_DEPTH >= height
            }
            BlockState::Forgotten => false,
        });

        let entry = blocks.entry(block_hash);
        let state = match &entry {
            Entry::Occupied(known) => Some(known.get().state()),
            Entry::Vacant(_) => None,
        };
        if let Some(res) = state.and_then(BlockState::response) {
            return Err(res);
        }

        let outcome = BlockOutcome {
            cell: Arc::default(),
            announced: matches!(state, Some(BlockState::Announced | BlockState::Added)),
        };
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

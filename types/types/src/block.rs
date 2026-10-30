//! A Monero [`block::Block`] that remembers its hash.

use std::{
    io::{self, Read},
    ops::Deref,
    sync::OnceLock,
};

use monero_oxide::block;

/// A Monero [`block::Block`] that remembers its hash, calculating it on first use if not given.
#[derive(Clone, Debug)]
pub struct Block {
    /// The block.
    block: block::Block,
    /// The hash of [`Self::block`].
    hash: OnceLock<[u8; 32]>,
}

impl Block {
    /// Wrap a [`block::Block`].
    pub const fn new(block: block::Block) -> Self {
        Self {
            block,
            hash: OnceLock::new(),
        }
    }

    /// Wrap a [`block::Block`] whose hash is already known.
    ///
    /// `hash` must be the block's hash.
    pub fn with_hash(block: block::Block, hash: [u8; 32]) -> Self {
        debug_assert_eq!(block.hash(), hash);

        Self {
            block,
            hash: OnceLock::from(hash),
        }
    }

    /// Read a [`block::Block`].
    ///
    /// # Errors
    /// This errors if the block is malformed.
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        Ok(Self::new(block::Block::read(r)?))
    }

    /// The block's hash.
    pub fn hash(&self) -> [u8; 32] {
        *self.hash.get_or_init(|| self.block.hash())
    }

    /// Unwrap the [`block::Block`].
    pub fn into_inner(self) -> block::Block {
        self.block
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        self.block == other.block
    }
}

impl Eq for Block {}

impl Deref for Block {
    type Target = block::Block;

    fn deref(&self) -> &block::Block {
        &self.block
    }
}

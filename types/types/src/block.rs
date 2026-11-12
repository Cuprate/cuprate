//! A Monero [`block::Block`] with its hash.

use std::{
    io::{self, Read},
    ops::Deref,
};

use monero_oxide::block;

/// A Monero [`block::Block`] with its hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// The block.
    block: block::Block,
    /// The hash of [`Self::block`].
    hash: [u8; 32],
}

impl Block {
    /// Wrap a [`block::Block`], calculating its hash.
    pub fn new(block: block::Block) -> Self {
        Self {
            hash: block.hash(),
            block,
        }
    }

    /// Wrap a [`block::Block`] whose hash is already known.
    ///
    /// `hash` must be the block's hash.
    pub fn with_hash(block: block::Block, hash: [u8; 32]) -> Self {
        debug_assert_eq!(block.hash(), hash);

        Self { block, hash }
    }

    /// Read a [`block::Block`].
    ///
    /// # Errors
    /// This errors if the block is malformed.
    pub fn read<R: Read>(r: &mut R) -> io::Result<Self> {
        block::Block::read(r).map(Self::new)
    }

    /// The block's hash.
    pub const fn hash(&self) -> [u8; 32] {
        self.hash
    }

    /// Unwrap the [`block::Block`].
    pub fn into_inner(self) -> block::Block {
        self.block
    }
}

impl Deref for Block {
    type Target = block::Block;

    fn deref(&self) -> &block::Block {
        &self.block
    }
}

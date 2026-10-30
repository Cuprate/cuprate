//! Cuprate FCMP++ Tree.
//!
//! This crate contains the implementation of the FCMP++ tree builder defined in [this paper](https://github.com/kayabaNerve/fcmp-plus-plus-paper).

mod pedersen_hash;
#[cfg(feature = "tapes")]
mod tapes_tree;
mod traits;
mod tree_builder;

pub type Branch = [u8; 32];

#[cfg(feature = "tapes")]
pub use tapes_tree::{FcmpTreeStorageConfig, TapeBlockInfo, TapesTreeStore};
pub use traits::{ChildReader, LayerWriter, NewLeaves, TreeStore};
pub use tree_builder::{CachedFcmpTreeRootData, FcmpRoot, TreeBuilder};

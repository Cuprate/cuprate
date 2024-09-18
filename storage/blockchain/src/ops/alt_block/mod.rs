//! Alternative Block/Chain Ops
//!
//! Alternative chains are chains that potentially have more proof-of-work than the main-chain
//! which we are tracking to potentially re-org to.
//!
//! Cuprate uses an ID system for alt-chains. When a split is made from the main-chain we generate
//! a random [`ChainID`](cuprate_types::ChainId) and assign it to the chain:
//!
//! ```text
//!      |
//!      |
//!      |   split
//!      |-------------
//!      |            |
//!      |            |
//!     \|/          \|/
//!  main-chain    ChainID(X)
//! ```
//!
//! In that example if we were to receive an alt-block which immediately follows the top block of `ChainID(X)`
//! then that block will also be stored under `ChainID(X)`. However, if it follows from another block from `ChainID(X)`
//! we will split into a chain with a different ID:
//!
//! ```text
//!      |
//!      |
//!      |   split
//!      |-------------
//!      |            |   split
//!      |            |-------------|
//!      |            |             |
//!      |            |             |
//!      |            |             |
//!     \|/          \|/           \|/
//!  main-chain    ChainID(X)    ChainID(Z)
//! ```
//!
//! As you can see if we wanted to get all the alt-blocks in `ChainID(Z)` that now includes some blocks from `ChainID(X)` as well.
//! [`get_alt_chain_history_ranges`] covers this and is the method to get the ranges of heights needed from each [`ChainID`](cuprate_types::ChainId)
//! to get all the alt-blocks in a given [`ChainID`](cuprate_types::ChainId).
//!
//! Although this should be kept in mind as a possibility, because Cuprate's block downloader will only track a single chain it is
//! unlikely that we will be tracking [`ChainID`](cuprate_types::ChainId)s that don't immediately connect to the main-chain.
//!
//! ## Why not use the block's `previous` field?
//!
//! Although that would be easier, it makes getting a range of block extremely slow, as we have to build the weight cache to verify
//! blocks, roughly 100,000 block headers needed, this cost is too high.
mod block;
mod chain;
mod tx;

pub use block::{
    add_alt_block, get_alt_block, get_alt_block_extended_header_from_height, get_alt_block_hash,
};
pub use chain::{get_alt_chain_history_ranges, update_alt_chain_info};
pub use tx::{add_alt_transaction_blob, get_alt_transaction};

/// Flush all alt-block data from all the alt-block tables.
///
/// This function completely empties the alt block tables.
pub fn flush_alt_blocks<'a, E: cuprate_database::EnvInner<'a>>(
    env_inner: &E,
    tx_rw: &mut E::Rw<'_>,
) -> Result<(), cuprate_database::RuntimeError> {
    use crate::tables::{
        AltBlockBlobs, AltBlockHeights, AltBlocksInfo, AltChainInfos, AltTransactionBlobs,
        AltTransactionInfos,
    };

    env_inner.clear_db::<AltChainInfos>(tx_rw)?;
    env_inner.clear_db::<AltBlockHeights>(tx_rw)?;
    env_inner.clear_db::<AltBlocksInfo>(tx_rw)?;
    env_inner.clear_db::<AltBlockBlobs>(tx_rw)?;
    env_inner.clear_db::<AltTransactionBlobs>(tx_rw)?;
    env_inner.clear_db::<AltTransactionInfos>(tx_rw)
}

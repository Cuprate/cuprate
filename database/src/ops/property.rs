//! Properties.

//---------------------------------------------------------------------------------------------------- Import
use monero_pruning::PruningSeed;

use crate::{
    error::RuntimeError,
    ops::macros::{doc_add_block_inner_invariant, doc_error},
};
//---------------------------------------------------------------------------------------------------- Free Functions
/// TODO
///
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::tx::*};
/// // TODO
/// ```
#[inline]
pub const fn get_blockchain_pruning_seed() -> Result<PruningSeed, RuntimeError> {
    // TODO: impl pruning.
    // We need a DB properties table.
    Ok(PruningSeed::NotPruned)
}

/// TODO
///
#[doc = doc_add_block_inner_invariant!()]
#[doc = doc_error!()]
///
/// # Example
/// ```rust
/// # use cuprate_database::{*, tables::*, ops::block::*, ops::tx::*};
/// // TODO
/// ```
#[inline]
pub const fn db_version() -> Result<u64, RuntimeError> {
    // TODO: We need a DB properties table.
    Ok(crate::constants::DATABASE_VERSION)
}

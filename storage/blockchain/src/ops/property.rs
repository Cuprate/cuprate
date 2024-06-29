//! Database properties functions - version, pruning, etc.
//!
//! SOMEDAY: the database `properties` table is not yet implemented.

//---------------------------------------------------------------------------------------------------- Import
use cuprate_pruning::PruningSeed;

use cuprate_database::RuntimeError;

use crate::ops::macros::doc_error;

//---------------------------------------------------------------------------------------------------- Free Functions
/// SOMEDAY
///
#[doc = doc_error!()]
///
/// # Example
/// ```rust
/// # use cuprate_blockchain::{*, tables::*, ops::block::*, ops::tx::*};
/// // SOMEDAY
/// ```
#[inline]
pub const fn get_blockchain_pruning_seed() -> Result<PruningSeed, RuntimeError> {
    // SOMEDAY: impl pruning.
    // We need a DB properties table.
    Ok(PruningSeed::NotPruned)
}

/// SOMEDAY
///
#[doc = doc_error!()]
///
/// # Example
/// ```rust
/// # use cuprate_blockchain::{*, tables::*, ops::block::*, ops::tx::*};
/// // SOMEDAY
/// ```
#[inline]
pub const fn db_version() -> Result<u64, RuntimeError> {
    // SOMEDAY: We need a DB properties table.
    Ok(crate::constants::DATABASE_VERSION)
}

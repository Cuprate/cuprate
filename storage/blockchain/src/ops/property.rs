//! Database properties functions - version, pruning, etc.
//!
//! SOMEDAY: the database `properties` table is not yet implemented.

//---------------------------------------------------------------------------------------------------- Import
use crate::error::DbResult;
use crate::ops::macros::doc_error;
use cuprate_pruning::PruningSeed;

//---------------------------------------------------------------------------------------------------- Free Functions
/// SOMEDAY
///
#[doc = doc_error!()]
///
#[inline]
pub const fn get_blockchain_pruning_seed() -> DbResult<PruningSeed> {
    // SOMEDAY: impl pruning.
    // We need a DB properties table.
    Ok(PruningSeed::NotPruned)
}

/// SOMEDAY
///
#[doc = doc_error!()]
///
#[inline]
pub const fn db_version() -> DbResult<u64> {
    // SOMEDAY: We need a DB properties table.
    Ok(crate::constants::DATABASE_VERSION)
}

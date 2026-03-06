//! Database properties functions - version, pruning, etc.
//!
//! SOMEDAY: the database `properties` table is not yet implemented.

use cuprate_pruning::PruningSeed;

use crate::error::DbResult;

//---------------------------------------------------------------------------------------------------- Free Functions
/// SOMEDAY
///
#[inline]
pub const fn get_blockchain_pruning_seed() -> DbResult<PruningSeed> {
    // SOMEDAY: impl pruning.
    // We need a DB properties table.
    Ok(PruningSeed::NotPruned)
}

/// SOMEDAY
///
#[inline]
pub const fn db_version() -> DbResult<u64> {
    // SOMEDAY: We need a DB properties table.
    Ok(crate::constants::DATABASE_VERSION)
}

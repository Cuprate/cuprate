//! General constants used throughout `cuprate-blockchain`.

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Version
/// Current major version of the database.
///
/// Returned by [`crate::ops::property::db_version`].
///
/// This is incremented by 1 when `cuprate_blockchain`'s
/// structure/schema/tables change.
///
/// This is akin to `VERSION` in `monerod`:
/// <https://github.com/monero-project/monero/blob/c8214782fb2a769c57382a999eaf099691c836e7/src/blockchain_db/lmdb/db_lmdb.cpp#L57>
pub const DATABASE_VERSION: u64 = 0;

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}

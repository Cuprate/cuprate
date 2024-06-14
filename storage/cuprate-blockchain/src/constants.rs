//! General constants used throughout `cuprate-blockchain`.

//---------------------------------------------------------------------------------------------------- Import
use cfg_if::cfg_if;

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

//---------------------------------------------------------------------------------------------------- Error Messages
/// Corrupt database error message.
///
/// The error message shown to end-users in panic
/// messages if we think the database is corrupted.
///
/// This is meant to be user-friendly.
pub const DATABASE_CORRUPT_MSG: &str = r"Cuprate has encountered a fatal error. The database may be corrupted.

TODO: instructions on:
1. What to do
2. How to fix (re-sync, recover, etc)
3. General advice for preventing corruption
4. etc";

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}

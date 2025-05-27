//! General constants used throughout `cuprate-blockchain`.

//---------------------------------------------------------------------------------------------------- Import
use cfg_if::cfg_if;

//---------------------------------------------------------------------------------------------------- Error Messages
/// Corrupt database error message.
///
/// The error message shown to end-users in panic
/// messages if we think the database is corrupted.
///
/// This is meant to be user-friendly.
pub const DATABASE_CORRUPT_MSG: &str = r"`cuprated` has encountered a fatal error. The database may be corrupted.

If `cuprated` continues to crash with the current database,
you may have to delete the database file and re-sync from scratch.

See <https://user.cuprate.org/resources/disk.html>
for more information on where database files are.

If this happens frequently, consider using the `Safe` sync mode.";

//---------------------------------------------------------------------------------------------------- Misc
/// Static string of the `crate` being used as the database backend.
///
/// | Backend | Value |
/// |---------|-------|
/// | `heed`  | `"heed"`
/// | `redb`  | `"redb"`
pub const DATABASE_BACKEND: &str = {
    cfg_if! {
        if #[cfg(all(feature = "redb", not(feature = "heed")))] {
            "redb"
        } else {
            "heed"
        }
    }
};

/// Cuprate's database filename.
///
/// Used in [`Config::db_file`](crate::config::Config::db_file).
///
/// | Backend | Value |
/// |---------|-------|
/// | `heed`  | `"data.mdb"`
/// | `redb`  | `"data.redb"`
pub const DATABASE_DATA_FILENAME: &str = {
    cfg_if! {
        if #[cfg(all(feature = "redb", not(feature = "heed")))] {
            "data.redb"
        } else {
            "data.mdb"
        }
    }
};

/// Cuprate's database lock filename.
///
/// | Backend | Value |
/// |---------|-------|
/// | `heed`  | `Some("lock.mdb")`
/// | `redb`  | `None` (redb doesn't use a file lock)
pub const DATABASE_LOCK_FILENAME: Option<&str> = {
    cfg_if! {
        if #[cfg(all(feature = "redb", not(feature = "heed")))] {
            None
        } else {
            Some("lock.mdb")
        }
    }
};

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}

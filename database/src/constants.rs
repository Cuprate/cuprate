//! General constants used throughout `cuprate-database`.

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Directory/Files
/// The directory that contains database-related files.
///
/// This is a sub-directory within the Cuprate folder, e.g:
/// ```txt
/// ~/.local/share/cuprate/
/// ├─ database/ # <-
///    ├─ data.mdb
///    ├─ lock.mdb
/// ```
pub const CUPRATE_DATABASE_DIR: &str = "database";

/// The actual database file name.
///
/// This is a _file_ within [`CUPRATE_DATABASE_DIR`], e.g:
/// ```txt
/// ~/.local/share/cuprate/
/// ├─ database/
///    ├─ data.mdb # <-
///    ├─ lock.mdb
/// ```
pub const CUPRATE_DATABASE_FILE: &str = "data";

// TODO: use `cuprate_helper` and crate OnceLock+fn for CUPRATE_DATABASE_DIR.

//---------------------------------------------------------------------------------------------------- Error Messages
/// Corrupt database error message.
///
/// The error message shown to end-users in panic
/// messages if we think the database is corrupted.
///
/// This is meant to be user-friendly.
pub const CUPRATE_DATABASE_CORRUPT_MSG: &str = r"Cuprate has encountered a fatal error. The database may be corrupted.

TODO: instructions on:
1. What to do
2. How to fix (re-sync, recover, etc)
3. General advice for preventing corruption
4. etc";

//---------------------------------------------------------------------------------------------------- Misc
cfg_if::cfg_if! {
    // If both backends are enabled, fallback to `heed`.
    // This is useful when using `--all-features`.
    if #[cfg(all(feature = "sanakirja", not(feature = "heed")))] {
        /// Static string of the `crate` being used as the database backend.
        pub const DATABASE_BACKEND: &str = "sanakirja";
    } else {
        /// Static string of the `crate` being used as the database backend.
        pub const DATABASE_BACKEND: &str = "heed";
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    /// Sanity check that our PATHs aren't empty... (will cause disaster).
    fn non_empty_path() {
        assert!(!CUPRATE_DATABASE_DIR.is_empty());
        assert!(!CUPRATE_DATABASE_FILE.is_empty());
    }
}

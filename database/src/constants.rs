//! General constants used throughout `cuprate-database`.

//---------------------------------------------------------------------------------------------------- Import

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

//---------------------------------------------------------------------------------------------------- Misc
cfg_if::cfg_if! {
    // If both backends are enabled, fallback to `heed`.
    // This is useful when using `--all-features`.
    if #[cfg(all(feature = "redb", not(feature = "heed")))] {
        /// Static string of the `crate` being used as the database backend.
        pub const DATABASE_BACKEND: &str = "redb";

        /// Cuprate's database filename.
        ///
        /// This is the filename for Cuprate's database, used in [`Config::db_file`](crate::config::Config::db_file).
        pub const DATABASE_FILENAME: &str = "data.redb";
    } else {
        /// Static string of the `crate` being used as the database backend.
        pub const DATABASE_BACKEND: &str = "heed";

        /// Cuprate's database filename.
        ///
        /// This is the filename for Cuprate's database, used in [`Config::db_file`](crate::config::Config::db_file).
        pub const DATABASE_FILENAME: &str = "data.mdb";
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}

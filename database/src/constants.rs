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
    if #[cfg(all(feature = "mdbx", not(feature = "heed")))] {
        /// Static string of the `crate` being used as the database backend.
        pub const DATABASE_BACKEND: &str = "mdbx";

        /// Cuprate's database filename.
        ///
        /// This is the filename for Cuprate's database, used in [`Config::db_file_path`](crate::config::Config::db_file_path).
        ///
        /// Reference: <https://libmdbx.dqdkfa.ru/group__c__api.html#gaea0edfb8c722071d05f8553598f13568>
        pub const DATABASE_FILENAME: &str = "mdbx.dat";
    } else {
        /// Static string of the `crate` being used as the database backend.
        pub const DATABASE_BACKEND: &str = "heed";

        /// Cuprate's database filename.
        ///
        /// This is the filename for Cuprate's database, used in [`Config::db_file_path`](crate::config::Config::db_file_path).
        ///
        /// Reference: <http://www.lmdb.tech/doc/group__internal.html#gad5a54432b85530e3f2cf9b88488e0eee>
        pub const DATABASE_FILENAME: &str = "data.mdb";
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}

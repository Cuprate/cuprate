//! General constants used throughout `cuprate-database`.

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Constants
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
    // use super::*;
}

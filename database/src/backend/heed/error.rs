//! Conversion from `heed::Error` -> `cuprate_database::RuntimeError`.

//---------------------------------------------------------------------------------------------------- Constants
/// The error message shown to end-users in panic
/// messages if we think database is corrupted.
///
/// This is meant to be user-friendly.
const CORRUPTION_ERROR_MSG: &str = r"Cuprate has encountered a fatal error. The database may be corrupted.

TODO: instructions on what to do to fix, general advice, etc";

//---------------------------------------------------------------------------------------------------- InitError
impl From<heed::Error> for crate::InitError {
    fn from(error: heed::Error) -> Self {
        use heed::Error as E1;
        use heed::MdbError as E2;

        // Reference of all possible errors `heed` will return
        // upon using [`heed::EnvOpenOptions::open`]:
        // <https://docs.rs/heed/latest/src/heed/env.rs.html#149-219>
        match error {
            E1::Io(io_error) => Self::Io(io_error),
            E1::DatabaseClosing => Self::ShuttingDown,

            // LMDB errors.
            E1::Mdb(mdb_error) => match mdb_error {
                E2::Invalid => Self::Invalid,
                E2::VersionMismatch => Self::InvalidVersion,
                E2::Other(c_int) => Self::Unknown(Box::new(mdb_error)),

                // "Located page was wrong type".
                // <https://docs.rs/heed/latest/heed/enum.MdbError.html#variant.Corrupted>
                //
                // "Requested page not found - this usually indicates corruption."
                // <https://docs.rs/heed/latest/heed/enum.MdbError.html#variant.PageNotFound>
                E2::Corrupted | E2::PageNotFound => Self::Corrupt,

                // These errors shouldn't be returned on database init.
                E2::Incompatible
                | E2::BadTxn
                | E2::Problem
                | E2::KeyExist
                | E2::NotFound
                | E2::MapFull
                | E2::ReadersFull
                | E2::PageFull
                | E2::DbsFull
                | E2::TlsFull
                | E2::TxnFull
                | E2::CursorFull
                | E2::MapResized
                | E2::BadRslot
                | E2::BadValSize
                | E2::BadDbi
                | E2::Panic => Self::Unknown(Box::new(mdb_error)),
            },

            E1::InvalidDatabaseTyping
            | E1::BadOpenOptions { .. }
            | E1::Encoding(_)
            | E1::Decoding(_) => Self::Unknown(Box::new(error)),
        }
    }
}

//---------------------------------------------------------------------------------------------------- RuntimeError
#[allow(clippy::fallible_impl_from)] // We need to panic sometimes.
impl From<heed::Error> for crate::RuntimeError {
    /// # Panics
    /// This will panic on unrecoverable errors for safety.
    fn from(error: heed::Error) -> Self {
        use heed::Error as E1;
        use heed::MdbError as E2;

        match error {
            // I/O errors.
            E1::Io(io_error) => Self::Io(io_error),

            // LMDB errors.
            E1::Mdb(mdb_error) => match mdb_error {
                E2::KeyExist => Self::KeyExists,
                E2::NotFound => Self::KeyNotFound,

                // Corruption errors, these have special panic messages.
                //
                // "Located page was wrong type".
                // <https://docs.rs/heed/latest/heed/enum.MdbError.html#variant.Corrupted>
                //
                // "Requested page not found - this usually indicates corruption."
                // <https://docs.rs/heed/latest/heed/enum.MdbError.html#variant.PageNotFound>
                E2::Corrupted | E2::PageNotFound => panic!("{mdb_error:?}\n{CORRUPTION_ERROR_MSG}"),

                // These errors should not occur, and if they do,
                // the best thing `cuprate_database` can to for
                // safety is to panic right here.
                E2::Panic
                | E2::PageFull
                | E2::Other(_)
                | E2::BadTxn
                | E2::Problem
                | E2::Invalid
                | E2::TlsFull
                | E2::TxnFull
                | E2::BadRslot
                | E2::VersionMismatch
                | E2::BadDbi => panic!("{mdb_error:?}"),

                // These errors are the same as above, but instead
                // of being errors we can't control, these are errors
                // that only happen if we write incorrect code.
                E2::MapFull        // Resize the map when needed.
                | E2::ReadersFull  // Don't spawn too many reader threads.
                | E2::DbsFull      // Don't create too many database tables.
                | E2::CursorFull   // Don't do crazy multi-nested LMDB cursor stuff.
                | E2::MapResized   // Resize the map when needed.
                | E2::Incompatible // <https://docs.rs/heed/0.20.0-alpha.9/heed/enum.MdbError.html#variant.Incompatible>
                | E2::BadValSize   // Unsupported size of key/DB name/data, or wrong DUP_FIXED size.
                    => panic!("fix the database code! {mdb_error:?}"),
            },

            // Database is shutting down.
            E1::DatabaseClosing => Self::ShuttingDown,

            // Only if we write incorrect code.
            E1::InvalidDatabaseTyping
            | E1::BadOpenOptions { .. }
            | E1::Encoding(_)
            | E1::Decoding(_) => panic!("fix the database code! {error:?}"),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

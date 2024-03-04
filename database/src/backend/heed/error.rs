//! Conversion from `heed::Error` -> `cuprate_database`'s errors.

//---------------------------------------------------------------------------------------------------- Use
use crate::constants::DATABASE_CORRUPT_MSG;

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
                E2::MapFull => Self::ResizeNeeded,

                // Corruption errors, these have special panic messages.
                //
                // "Located page was wrong type".
                // <https://docs.rs/heed/latest/heed/enum.MdbError.html#variant.Corrupted>
                //
                // "Requested page not found - this usually indicates corruption."
                // <https://docs.rs/heed/latest/heed/enum.MdbError.html#variant.PageNotFound>
                E2::Corrupted | E2::PageNotFound => panic!("{mdb_error:#?}\n{DATABASE_CORRUPT_MSG}"),

                // These errors should not occur, and if they do,
                // the best thing `cuprate_database` can do for
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
                | E2::BadDbi => panic!("{mdb_error:#?}"),

                // These errors are the same as above, but instead
                // of being errors we can't control, these are errors
                // that only happen if we write incorrect code.

                // "Database contents grew beyond environment mapsize."
                // We should be resizing the map when needed, this error
                // occurring indicates we did _not_ do that, which is a bug
                // and we should panic.
                //
                // TODO: This can also mean _another_ process wrote to our
                // LMDB file and increased the size. I don't think we need to accommodate for this.
                // <http://www.lmdb.tech/doc/group__mdb.html#gaa2506ec8dab3d969b0e609cd82e619e5>
                // Although `monerod` reacts to that instead of `MDB_MAP_FULL`
                // which is what `mdb_put()` returns so... idk?
                // <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L526>
                | E2::MapResized
                // We should be setting `heed::EnvOpenOptions::max_readers()`
                // with our reader thread value in [`crate::config::Config`],
                // thus this error should never occur.
                // <http://www.lmdb.tech/doc/group__mdb.html#gae687966c24b790630be2a41573fe40e2>
                | E2::ReadersFull
                // Do not open more database tables than we initially started with.
                // We know this number at compile time (amount of `Table`'s) so this
                // should never happen.
                // <https://docs.rs/heed/0.20.0-alpha.9/heed/struct.EnvOpenOptions.html#method.max_dbs>
                // <https://docs.rs/heed/0.20.0-alpha.9/src/heed/env.rs.html#251>
                | E2::DbsFull
                // Don't do crazy multi-nested LMDB cursor stuff.
                | E2::CursorFull
                // <https://docs.rs/heed/0.20.0-alpha.9/heed/enum.MdbError.html#variant.Incompatible>
                | E2::Incompatible
                // Unsupported size of key/DB name/data, or wrong DUP_FIXED size.
                // Don't use a key that is `>511` bytes.
                // <http://www.lmdb.tech/doc/group__mdb.html#gaaf0be004f33828bf2fb09d77eb3cef94>
                | E2::BadValSize
                    => panic!("fix the database code! {mdb_error:#?}"),
            },

            // Only if we write incorrect code.
            E1::InvalidDatabaseTyping
            | E1::DatabaseClosing
            | E1::BadOpenOptions { .. }
            | E1::Encoding(_)
            | E1::Decoding(_) => panic!("fix the database code! {error:#?}"),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

//! Conversion from `heed::Error` -> `cuprate_database::RuntimeError`.

//---------------------------------------------------------------------------------------------------- Import
use std::borrow::Cow;

//---------------------------------------------------------------------------------------------------- Error
impl From<heed::Error> for crate::RuntimeError {
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
                E2::VersionMismatch => Self::VersionMismatch,
                E2::MapFull => Self::MapFull,
                E2::ReadersFull => Self::ReadersFull,
                E2::PageFull => Self::PageFull,
                E2::Other(c_int) => Self::Unknown(Cow::Owned(format!("{mdb_error:?}"))),

                // "Located page was wrong type".
                // <https://docs.rs/heed/latest/heed/enum.MdbError.html#variant.Corrupted>
                //
                // "Requested page not found - this usually indicates corruption."
                // <https://docs.rs/heed/latest/heed/enum.MdbError.html#variant.PageNotFound>
                E2::Corrupted | E2::PageNotFound => Self::Corrupt,

                // "Update of meta page failed or environment had fatal error."
                // <https://docs.rs/sanakirja/latest/sanakirja/enum.Error.html#variant.Poison>
                //
                // If LMDB itself fails, should we even try to recover?
                E2::Panic => unreachable!(),

                // TODO: these are recoverable?
                E2::BadTxn | E2::Problem => Self::Other(Cow::from(format!("{mdb_error:?}"))),

                // TODO: are these all unrecoverable/unreachable errors?
                E2::DbsFull => panic!(mdb_error), // We know the DB count at compile time.
                E2::Invalid => panic!(mdb_error), // This is an `InitError`, it cannot occur here
                E2::TlsFull => panic!(mdb_error), // ???
                E2::TxnFull => panic!(mdb_error), // ???
                E2::CursorFull => panic!(mdb_error), // Shouldn't happen unless we do crazy cursor stuff (we don't)
                E2::MapResized => panic!(mdb_error), // We should be properly handling resizes, so this should panic indicating a bug
                E2::Incompatible => panic!(mdb_error), // Should never happen
                E2::BadRslot => panic!(mdb_error),   // ???
                E2::BadValSize => panic!(mdb_error), // Should never happen
                E2::BadDbi => panic!(mdb_error),     // ???
            },

            // Database is shutting down.
            E1::DatabaseClosing => Self::ShuttingDown,

            // TODO: these will never occur once correct?
            // TODO: (de)serialization is infallible?
            E1::InvalidDatabaseTyping
            | E1::BadOpenOptions { .. }
            | E1::Encoding(_)
            | E1::Decoding(_) => panic!(mdb_error),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

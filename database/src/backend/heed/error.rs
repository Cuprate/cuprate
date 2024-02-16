//! Conversion from `heed::Error` -> `cuprate_database::RuntimeError`.

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
                | E2::Panic => Self::Fatal(Box::new(mdb_error)),
            },

            // TODO: these will never occur once correct?
            // TODO: (de)serialization is infallible?
            E1::InvalidDatabaseTyping
            | E1::BadOpenOptions { .. }
            | E1::Encoding(_)
            | E1::Decoding(_) => Self::Fatal(Box::new(error)),
        }
    }
}

//---------------------------------------------------------------------------------------------------- RuntimeError
impl From<heed::Error> for crate::RuntimeError {
    fn from(error: heed::Error) -> Self {
        use heed::Error as E1;
        use heed::MdbError as E2;

        #[allow(clippy::match_same_arms)] // TODO: remove me.
        match error {
            // I/O errors.
            E1::Io(io_error) => Self::Io(io_error),

            // LMDB errors.
            #[allow(clippy::match_same_arms)] // TODO: remove me.
            E1::Mdb(mdb_error) => match mdb_error {
                E2::KeyExist => Self::KeyExists,
                E2::NotFound => Self::KeyNotFound,
                E2::MapFull => Self::MapFull,
                E2::ReadersFull => Self::ReadersFull,
                E2::PageFull => Self::PageFull,
                E2::Other(c_int) => Self::Unknown(Box::new(mdb_error)),

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
                E2::Panic => Self::Fatal(Box::new(mdb_error)),

                // TODO: are these are recoverable?
                E2::BadTxn | E2::Problem => Self::TxMustAbort,

                // TODO: are these all unrecoverable/unreachable errors?
                E2::DbsFull // We know the DB count at compile time.
                | E2::Invalid // This is an `InitError`, it cannot occur here
                | E2::TlsFull // ???
                | E2::TxnFull // ???
                | E2::CursorFull // Shouldn't happen unless we do crazy cursor stuff (we don't)
                | E2::MapResized // We should be properly handling resizes, so this should panic indicating a bug
                | E2::Incompatible // Should never happen
                | E2::BadRslot   // ???
                | E2::BadValSize // Should never happen
                | E2::VersionMismatch // This is an `InitError`
                | E2::BadDbi => Self::Fatal(Box::new(mdb_error)), // ???
            },

            // Database is shutting down.
            E1::DatabaseClosing => Self::ShuttingDown,

            // TODO: these will never occur once correct?
            // TODO: (de)serialization is infallible?
            E1::InvalidDatabaseTyping
            | E1::BadOpenOptions { .. }
            | E1::Encoding(_)
            | E1::Decoding(_) => Self::Fatal(Box::new(error)),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

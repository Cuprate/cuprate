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
                E2::Other(c_int) => {
                    Self::Unknown(Cow::Owned(format!("heed::Error::Other({c_int})")))
                }
                E2::DbsFull
                | E2::PageNotFound
                | E2::Corrupted
                | E2::Panic
                | E2::Invalid
                | E2::TlsFull
                | E2::TxnFull
                | E2::CursorFull
                | E2::MapResized
                | E2::Incompatible
                | E2::BadRslot
                | E2::BadTxn
                | E2::BadValSize
                | E2::BadDbi
                | E2::Problem => Self::Unknown(Cow::from(std::any::type_name_of_val(&mdb_error))),
            },

            // Database is shutting down.
            E1::DatabaseClosing => Self::ShuttingDown,

            // TODO: these will never occur once correct?
            // TODO: (de)serialization is infallible?
            E1::InvalidDatabaseTyping
            | E1::BadOpenOptions { .. }
            | E1::Encoding(_)
            | E1::Decoding(_) => unreachable!(),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

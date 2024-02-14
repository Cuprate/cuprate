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

                // TODO: what to do with these errors?
                E2::DbsFull => todo!(),
                E2::PageNotFound => todo!(),
                E2::Corrupted => todo!(),
                E2::Panic => todo!(),
                E2::Invalid => todo!(),
                E2::TlsFull => todo!(),
                E2::TxnFull => todo!(),
                E2::CursorFull => todo!(),
                E2::MapResized => todo!(),
                E2::Incompatible => todo!(),
                E2::BadRslot => todo!(),
                E2::BadTxn => todo!(),
                E2::BadValSize => todo!(),
                E2::BadDbi => todo!(),
                E2::Problem => todo!(),
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

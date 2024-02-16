//! Conversion from `sanakirja::Error` -> `cuprate_database::RuntimeError`.

//---------------------------------------------------------------------------------------------------- Import
use crate::constants::CUPRATE_DATABASE_CORRUPT_MSG;

//---------------------------------------------------------------------------------------------------- InitError
impl From<sanakirja::Error> for crate::InitError {
    fn from(error: sanakirja::Error) -> Self {
        use sanakirja::Error as E;

        match error {
            E::IO(io_error) => Self::Io(io_error),
            E::VersionMismatch => Self::InvalidVersion,

            // A CRC failure essentially  means a `sanakirja` page was corrupt.
            // <https://docs.rs/sanakirja/latest/sanakirja/enum.Error.html#variant.CRC>
            E::Corrupt(_) | E::CRC(_) => Self::Corrupt,

            // A database lock was poisoned.
            // <https://docs.rs/sanakirja/latest/sanakirja/enum.Error.html#variant.Poison>
            E::Poison => Self::Unknown(Box::new(error)),
        }
    }
}

//---------------------------------------------------------------------------------------------------- RuntimeError
#[allow(clippy::fallible_impl_from)] // We need to panic sometimes.
impl From<sanakirja::Error> for crate::RuntimeError {
    fn from(error: sanakirja::Error) -> Self {
        use sanakirja::Error as E;

        match error {
            E::IO(io_error) => Self::Io(io_error),

            // A CRC failure essentially  means a `sanakirja` page was corrupt.
            // <https://docs.rs/sanakirja/latest/sanakirja/enum.Error.html#variant.CRC>
            E::Corrupt(_) | E::CRC(_) => panic!("{error:?}\n{CUPRATE_DATABASE_CORRUPT_MSG}"),

            // These errors should not occur, and if they do,
            // the best thing `cuprate_database` can to for
            // safety is to panic right here.
            E::Poison | E::VersionMismatch => panic!("{error:?}"),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

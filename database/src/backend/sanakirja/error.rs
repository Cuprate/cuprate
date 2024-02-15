//! Conversion from `sanakirja::Error` -> `cuprate_database::RuntimeError`.

//---------------------------------------------------------------------------------------------------- Import

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
            E::Poison => Self::Fatal(Box::new(error)),
        }
    }
}

//---------------------------------------------------------------------------------------------------- RuntimeError
impl From<sanakirja::Error> for crate::RuntimeError {
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
            E::Poison => Self::Fatal(Box::new(error)),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

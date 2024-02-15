//! Conversion from `sanakirja::Error` -> `cuprate_database::RuntimeError`.

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Error
#[allow(clippy::fallible_impl_from)] // We need to panic sometimes
impl From<sanakirja::Error> for crate::RuntimeError {
    fn from(error: sanakirja::Error) -> Self {
        use sanakirja::Error as E;

        match error {
            E::IO(io_error) => Self::Io(io_error),
            E::VersionMismatch => Self::VersionMismatch,

            // A CRC failure essentially  means a `sanakirja` page was corrupt.
            // <https://docs.rs/sanakirja/latest/sanakirja/enum.Error.html#variant.CRC>
            E::Corrupt(_) | E::CRC(_) => Self::Corrupt,

            // A database lock was poisoned.
            // If 1 thread panics, everything should panic, so panic here.
            //
            // <https://docs.rs/sanakirja/latest/sanakirja/enum.Error.html#variant.Poison>
            E::Poison => panic!("sanakirja database lock poison"),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

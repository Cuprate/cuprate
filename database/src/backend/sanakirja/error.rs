//! Conversion from `sanakirja::Error` -> `cuprate_database::RuntimeError`.

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Error
impl From<sanakirja::Error> for crate::RuntimeError {
    fn from(error: sanakirja::Error) -> Self {
        use sanakirja::Error as E;

        match error {
            E::IO(io_error) => Self::Io(io_error),
            E::VersionMismatch => Self::VersionMismatch,

            // TODO: what to do with these errors?
            E::Poison => todo!(),
            E::CRC(error) => todo!(),
            E::Corrupt(u) => todo!(),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

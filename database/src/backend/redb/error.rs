//! Conversion from `redb::Error` -> `cuprate_database::RuntimeError`.

//---------------------------------------------------------------------------------------------------- Import
use crate::constants::DATABASE_CORRUPT_MSG;

//---------------------------------------------------------------------------------------------------- InitError
impl From<redb::Error> for crate::InitError {
    fn from(error: redb::Error) -> Self {
        use redb::Error as E;

        #[allow(clippy::match_single_binding)]
        match error {
            _ => todo!(),
        }
    }
}

//---------------------------------------------------------------------------------------------------- RuntimeError
#[allow(clippy::fallible_impl_from)] // We need to panic sometimes.
impl From<redb::Error> for crate::RuntimeError {
    fn from(error: redb::Error) -> Self {
        use redb::Error as E;

        #[allow(clippy::match_single_binding)]
        match error {
            _ => todo!(),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

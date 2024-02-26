//! Implementation of `trait RoTx/RwTx` for `sanakirja`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    error::RuntimeError,
    transaction::{RoTx, RwTx},
};

//---------------------------------------------------------------------------------------------------- RoTx
impl RoTx<'_> for libmdbx::Transaction<'_, libmdbx::RO, libmdbx::WriteMap> {
    fn commit(self) -> Result<(), RuntimeError> {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- RwTx
impl RwTx<'_> for libmdbx::Transaction<'_, libmdbx::RW, libmdbx::WriteMap> {
    /// TODO
    /// # Errors
    /// TODO
    fn commit(self) -> Result<(), RuntimeError> {
        todo!()
    }

    /// TODO
    fn abort(self) {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

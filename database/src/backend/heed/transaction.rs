//! Implementation of `trait TxRo/TxRw` for `heed`.

use std::{ops::Deref, sync::RwLockReadGuard};

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    error::RuntimeError,
    transaction::{TxRo, TxRw},
};

//---------------------------------------------------------------------------------------------------- TxRo
impl TxRo<'_> for heed::RoTxn<'_> {
    fn commit(self) -> Result<(), RuntimeError> {
        Ok(self.commit()?)
    }
}

//---------------------------------------------------------------------------------------------------- TxRw
impl TxRo<'_> for heed::RwTxn<'_> {
    fn commit(self) -> Result<(), RuntimeError> {
        self.commit().map_err(Into::into)
    }
}

impl TxRw<'_> for heed::RwTxn<'_> {
    fn commit(self) -> Result<(), RuntimeError> {
        self.commit().map_err(Into::into)
    }

    /// This function is infallible.
    fn abort(self) -> Result<(), RuntimeError> {
        self.abort();
        Ok(())
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

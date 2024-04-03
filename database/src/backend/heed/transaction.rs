//! Implementation of `trait TxRo/TxRw` for `heed`.

use std::{cell::UnsafeCell, ops::Deref, sync::RwLockReadGuard};

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
impl TxRo<'_> for UnsafeCell<heed::RwTxn<'_>> {
    fn commit(self) -> Result<(), RuntimeError> {
        Ok(self.into_inner().commit()?)
    }
}

impl TxRw<'_> for UnsafeCell<heed::RwTxn<'_>> {
    fn commit(self) -> Result<(), RuntimeError> {
        Ok(self.into_inner().commit()?)
    }

    /// This function is infallible.
    fn abort(self) -> Result<(), RuntimeError> {
        self.into_inner().abort();
        Ok(())
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

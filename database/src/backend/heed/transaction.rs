//! Implementation of `trait TxRo/TxRw` for `heed`.

use std::{ops::Deref, sync::RwLockReadGuard};

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    error::RuntimeError,
    transaction::{TxCreator, TxRo, TxRw},
};

//---------------------------------------------------------------------------------------------------- TxRo
impl<'env> TxCreator<'env, heed::RoTxn<'env>, heed::RwTxn<'env>>
    for RwLockReadGuard<'env, heed::Env>
{
    #[inline]
    fn tx_ro(&'env self) -> Result<heed::RoTxn<'env>, RuntimeError> {
        Ok(self.read_txn()?)
    }

    #[inline]
    fn tx_rw(&'env self) -> Result<heed::RwTxn<'env>, RuntimeError> {
        Ok(self.write_txn()?)
    }
}

//---------------------------------------------------------------------------------------------------- TxRo
impl TxRo<'_> for heed::RoTxn<'_> {
    fn commit(self) -> Result<(), RuntimeError> {
        self.commit().map_err(Into::into)
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

    fn abort(self) {
        self.abort();
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

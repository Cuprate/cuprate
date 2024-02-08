//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::{database::Database, env::Env, error::RuntimeError, table::Table};

use std::path::Path;

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- Env
/// A strongly typed, concrete database environment, backed by `heed`.
pub struct ConcreteEnv(heed::Env);

//---------------------------------------------------------------------------------------------------- Env Impl
impl Env for ConcreteEnv {
    /// TODO
    type RoTx<'db> = heed::RoTxn<'db>;

    /// TODO
    type RwTx<'db> = heed::RwTxn<'db>;

    //------------------------------------------------ Required
    #[cold]
    #[inline(never)] // called once.
    /// TODO
    /// # Errors
    /// TODO
    fn open<P: AsRef<Path>>(path: P) -> Result<Self, RuntimeError> {
        todo!()
    }

    /// TODO
    /// # Errors
    /// TODO
    fn sync(&self) -> Result<(), RuntimeError> {
        todo!()
    }

    #[inline]
    /// TODO
    /// # Errors
    /// TODO
    fn ro_tx(&self) -> Result<Self::RoTx<'_>, RuntimeError> {
        todo!()
    }

    #[inline]
    /// TODO
    /// # Errors
    /// TODO
    fn rw_tx(&self) -> Result<Self::RwTx<'_>, RuntimeError> {
        todo!()
    }

    #[cold]
    #[inline(never)] // called infrequently?.
    /// TODO
    /// # Errors
    /// TODO
    fn create_database<T: Table>(
        &self,
        tx_rw: &mut Self::RwTx<'_>,
    ) -> Result<impl Database<T>, RuntimeError> {
        let tx: heed::Database<T::Key, T::Value> = todo!();
        Ok(tx)
    }

    #[inline]
    /// TODO
    /// # Errors
    /// TODO
    fn open_database<T: Table>(
        &self,
        to_rw: &Self::RoTx<'_>,
    ) -> Result<Option<impl Database<T>>, RuntimeError> {
        let tx: heed::Database<T::Key, T::Value> = todo!();
        Ok(Some(tx))
    }
}

//---------------------------------------------------------------------------------------------------- Transaction Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

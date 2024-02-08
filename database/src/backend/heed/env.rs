//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    backend::heed::transaction::{ConcreteRoTx, ConcreteRwTx},
    env::Env,
    error::{InitError, RuntimeError},
    table::Table,
    transaction::{RoTx, RwTx},
};

use std::path::Path;

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- Heed
/// A strongly typed, concrete database environment, backed by `heed`.
///
pub struct ConcreteEnv(heed::Env);

//---------------------------------------------------------------------------------------------------- Heed Impl

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
    fn tx_ro(&self) -> Result<Self::RoTx<'_>, RuntimeError> {
        todo!()
    }

    #[inline]
    /// TODO
    /// # Errors
    /// TODO
    fn tx_rw(&self) -> Result<Self::RwTx<'_>, RuntimeError> {
        todo!()
    }

    #[cold]
    #[inline(never)] // called infrequently?.
    /// TODO
    /// # Errors
    /// TODO
    fn create_database<'db, T: Table + 'db>(
        &'db self,
        tx_rw: &'db mut Self::RwTx<'_>,
    ) -> Result<impl RwTx<'db, T::Key, T::Value>, RuntimeError> {
        let tx: ConcreteRwTx<T::Key, T::Value> = todo!();
        Ok(tx)
    }

    #[inline]
    /// TODO
    /// # Errors
    /// TODO
    fn open_database<'db, T: Table + 'db>(
        &'db self,
        to_rw: &'db Self::RoTx<'_>,
    ) -> Result<Option<impl RoTx<'db, T::Key, T::Value>>, RuntimeError> {
        let tx: ConcreteRoTx<T::Key, T::Value> = todo!();
        Ok(Some(tx))
    }
}

//---------------------------------------------------------------------------------------------------- Transaction Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

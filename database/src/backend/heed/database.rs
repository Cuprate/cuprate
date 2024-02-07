//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    backend::heed::transaction::{ConcreteRoTx, ConcreteRwTx},
    database::Database,
    error::{InitError, RuntimeError},
    table::Table,
    transaction::{RoTx, RwTx},
};

use std::path::Path;

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- Heed
/// TODO
pub struct ConcreteDatabase(heed::Env);

//---------------------------------------------------------------------------------------------------- Heed Impl

//---------------------------------------------------------------------------------------------------- Database Impl
impl Database for ConcreteDatabase {
    /// TODO
    type RoTx<'db> = heed::RoTxn<'db>;

    /// TODO
    type RwTx<'db> = heed::RwTxn<'db>;

    //------------------------------------------------ Required
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

    /// TODO
    /// # Errors
    /// TODO
    fn tx_ro(&self) -> Result<Self::RoTx<'_>, RuntimeError> {
        todo!()
    }

    /// TODO
    /// # Errors
    /// TODO
    fn tx_rw(&self) -> Result<Self::RwTx<'_>, RuntimeError> {
        todo!()
    }

    /// TODO
    /// # Errors
    /// TODO
    fn create_table<'db, T: Table + 'db>(
        &'db self,
        // tx_rw: &mut Self::RwTx<'_>,
    ) -> Result<ConcreteRwTx<'db, T::Key, T::Value>, RuntimeError> {
        todo!()
    }

    /// TODO
    /// # Errors
    /// TODO
    fn get_table<'db, T: Table + 'db>(
        &'db self,
        // to_rw: &mut Self::RoTx<'_>,
    ) -> Result<Option<ConcreteRoTx<'db, T::Key, T::Value>>, RuntimeError> {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- Transaction Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

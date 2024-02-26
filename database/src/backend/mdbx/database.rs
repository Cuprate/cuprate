//! Implementation of `trait Database` for `mdbx`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{backend::mdbx::types::MdbxDb, database::Database, error::RuntimeError, table::Table};

//---------------------------------------------------------------------------------------------------- Database Impls
impl<T: Table> Database<T> for MdbxDb<'_> {
    type RoTx<'db> = libmdbx::Transaction<'db, libmdbx::RO, libmdbx::WriteMap>;
    type RwTx<'db> = libmdbx::Transaction<'db, libmdbx::RW, libmdbx::WriteMap>;

    fn get(&self, ro_tx: &Self::RoTx<'_>, key: &T::Key) -> Result<Option<T::Value>, RuntimeError> {
        todo!()
    }

    fn get_range(
        &self,
        ro_tx: &Self::RoTx<'_>,
        key: &T::Key,
        amount: usize,
    ) -> Result<impl Iterator<Item = T::Value>, RuntimeError> {
        let iter: std::vec::Drain<'_, T::Value> = todo!();
        Ok(iter)
    }

    fn put(
        &mut self,
        rx_tx: &mut Self::RwTx<'_>,
        key: &T::Key,
        value: &T::Value,
    ) -> Result<(), RuntimeError> {
        todo!()
    }

    fn clear(&mut self, rx_tx: &mut Self::RwTx<'_>) -> Result<(), RuntimeError> {
        todo!()
    }

    fn delete(&mut self, rx_tx: &mut Self::RwTx<'_>, key: &T::Key) -> Result<bool, RuntimeError> {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

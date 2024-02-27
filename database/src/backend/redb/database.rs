//! Implementation of `trait Database` for `redb`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    backend::redb::types::{RedbTableRo, RedbTableRw},
    database::{DatabaseRead, DatabaseWrite},
    error::RuntimeError,
    table::Table,
};

//---------------------------------------------------------------------------------------------------- DatabaseRead
impl<T: Table> DatabaseRead<T> for RedbTableRo<'_> {
    fn get(&self, key: &T::Key) -> Result<Option<T::Value>, RuntimeError> {
        todo!()
    }

    fn get_range(
        &self,
        key: &T::Key,
        amount: usize,
    ) -> Result<impl Iterator<Item = T::Value>, RuntimeError> {
        let iter: std::vec::Drain<'_, T::Value> = todo!();
        Ok(iter)
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseWrite
impl<T: Table> DatabaseRead<T> for RedbTableRw<'_, '_> {
    fn get(&self, key: &T::Key) -> Result<Option<T::Value>, RuntimeError> {
        todo!()
    }

    fn get_range(
        &self,
        key: &T::Key,
        amount: usize,
    ) -> Result<impl Iterator<Item = T::Value>, RuntimeError> {
        let iter: std::vec::Drain<'_, T::Value> = todo!();
        Ok(iter)
    }
}

impl<T: Table> DatabaseWrite<T> for RedbTableRw<'_, '_> {
    fn put(&mut self, key: &T::Key, value: &T::Value) -> Result<(), RuntimeError> {
        todo!()
    }

    fn clear(&mut self) -> Result<(), RuntimeError> {
        todo!()
    }

    fn delete(&mut self, key: &T::Key) -> Result<bool, RuntimeError> {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

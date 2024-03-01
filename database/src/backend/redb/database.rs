//! Implementation of `trait DatabaseR{o,w}` for `redb`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    backend::redb::types::{RedbTableRo, RedbTableRw},
    database::{DatabaseRo, DatabaseRw},
    error::RuntimeError,
    table::Table,
};

//---------------------------------------------------------------------------------------------------- DatabaseRo
impl<T: Table> DatabaseRo<T> for RedbTableRo<'_, T::Key, T::Value> {
    fn get(&self, key: &T::Key) -> Result<Option<&T::Value>, RuntimeError> {
        todo!()
    }

    fn get_range<'a>(
        &'a self,
        key: &'a T::Key,
        amount: usize,
    ) -> Result<impl Iterator<Item = &'a T::Value>, RuntimeError>
    where
        <T as Table>::Value: 'a,
    {
        let iter: std::vec::Drain<'_, &T::Value> = todo!();
        Ok(iter)
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRw
impl<T: Table> DatabaseRo<T> for RedbTableRw<'_, '_, T::Key, T::Value> {
    fn get(&self, key: &T::Key) -> Result<Option<&T::Value>, RuntimeError> {
        todo!()
    }

    fn get_range<'a>(
        &'a self,
        key: &'a T::Key,
        amount: usize,
    ) -> Result<impl Iterator<Item = &'a T::Value>, RuntimeError>
    where
        <T as Table>::Value: 'a,
    {
        let iter: std::vec::Drain<'_, &T::Value> = todo!();
        Ok(iter)
    }
}

impl<T: Table> DatabaseRw<T> for RedbTableRw<'_, '_, T::Key, T::Value> {
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

//! Implementation of `trait Database` for `heed`.

//---------------------------------------------------------------------------------------------------- Import
use std::marker::PhantomData;

use crate::{
    backend::heed::types::HeedDb,
    database::{DatabaseRead, DatabaseWrite},
    error::RuntimeError,
    table::Table,
};

//---------------------------------------------------------------------------------------------------- Heed Database Wrappers
// TODO: document that this exists to match
// `redb`'s behavior of tying the lifetime of
// `tx`'s and the opened table.
//
// TODO: do we need this `T: Table` phantom bound?

/// TODO
/// Matches `redb::ReadOnlyTable`.
pub(super) struct HeedTableRo<'env, T: Table> {
    /// TODO
    db: HeedDb,
    /// TODO
    tx: &'env heed::RoTxn<'env>,
    /// TODO
    _table: PhantomData<T>,
}

/// TODO
/// Matches `redb::Table` (read & write).
pub(super) struct HeedTableRw<'env, T: Table> {
    /// TODO
    db: HeedDb,
    /// TODO
    tx: &'env mut heed::RwTxn<'env>,
    /// TODO
    _table: PhantomData<T>,
}

//---------------------------------------------------------------------------------------------------- DatabaseRead Impl
impl<T: Table> DatabaseRead<T> for HeedTableRo<'_, T> {
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

//---------------------------------------------------------------------------------------------------- DatabaseWrite Impl
impl<T: Table> DatabaseRead<T> for HeedTableRw<'_, T> {
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

impl<T: Table> DatabaseWrite<T> for HeedTableRw<'_, T> {
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

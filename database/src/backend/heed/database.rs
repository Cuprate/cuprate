//! Implementation of `trait Database` for `heed`.

//---------------------------------------------------------------------------------------------------- Import
use std::marker::PhantomData;

use crate::{
    backend::heed::types::HeedDb,
    database::{DatabaseRo, DatabaseRw},
    error::RuntimeError,
    table::Table,
};

//---------------------------------------------------------------------------------------------------- Heed Database Wrappers
// Q. Why does `HeedTableR{o,w}` exist?
// A. These wrapper types combine `heed`'s database/table
// types with its transaction types. It exists to match
// `redb`, which has this behavior built-in.
//
// `redb` forces us to abstract read/write semantics
// at the _opened table_ level, so, we must match that in `heed`,
// which abstracts it at the transaction level.
//
// We must also maintain the ability for
// write operations to also read, aka, `Rw`.
//
// TODO: do we need the `T: Table` phantom bound?
// It allows us to reference the `Table` info.

/// An opened read-only database associated with a transaction.
///
/// Matches `redb::ReadOnlyTable`.
pub(super) struct HeedTableRo<'env, T: Table> {
    /// An already opened database table.
    db: HeedDb,
    /// The associated read-only transaction that opened this table.
    tx_ro: &'env heed::RoTxn<'env>,
    /// TODO: do we need this?
    _table: PhantomData<T>,
}

/// An opened read/write database associated with a transaction.
///
/// Matches `redb::Table` (read & write).
pub(super) struct HeedTableRw<'env, T: Table> {
    /// TODO
    db: HeedDb,
    /// The associated read/write transaction that opened this table.
    tx_rw: &'env mut heed::RwTxn<'env>,
    /// TODO: do we need this?
    _table: PhantomData<T>,
}

//---------------------------------------------------------------------------------------------------- DatabaseRo Impl
impl<T: Table> DatabaseRo<T> for HeedTableRo<'_, T> {
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

//---------------------------------------------------------------------------------------------------- DatabaseRw Impl
impl<T: Table> DatabaseRo<T> for HeedTableRw<'_, T> {
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

impl<T: Table> DatabaseRw<T> for HeedTableRw<'_, T> {
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

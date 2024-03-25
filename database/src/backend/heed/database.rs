//! Implementation of `trait Database` for `heed`.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::{Borrow, Cow},
    fmt::Debug,
    ops::RangeBounds,
    sync::RwLockReadGuard,
};

use crate::{
    backend::heed::{storable::StorableHeed, types::HeedDb},
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

/// An opened read-only database associated with a transaction.
///
/// Matches `redb::ReadOnlyTable`.
pub(super) struct HeedTableRo<'tx, T: Table> {
    /// An already opened database table.
    pub(super) db: HeedDb<T::Key, T::Value>,
    /// The associated read-only transaction that opened this table.
    pub(super) tx_ro: &'tx heed::RoTxn<'tx>,
}

/// An opened read/write database associated with a transaction.
///
/// Matches `redb::Table` (read & write).
pub(super) struct HeedTableRw<'env, 'tx, T: Table> {
    /// An already opened database table.
    pub(super) db: HeedDb<T::Key, T::Value>,
    /// The associated read/write transaction that opened this table.
    pub(super) tx_rw: &'tx mut heed::RwTxn<'env>,
}

//---------------------------------------------------------------------------------------------------- Shared functions
// FIXME: we cannot just deref `HeedTableRw -> HeedTableRo` and
// call the functions since the database is held by value, so
// just use these generic functions that both can call instead.

/// Shared [`DatabaseRo::get()`].
#[inline]
fn get<T: Table>(
    db: &HeedDb<T::Key, T::Value>,
    tx_ro: &heed::RoTxn<'_>,
    key: &T::Key,
) -> Result<T::Value, RuntimeError> {
    db.get(tx_ro, key)?.ok_or(RuntimeError::KeyNotFound)
}

/// Shared [`DatabaseRo::get_range()`].
#[inline]
fn get_range<'a, T: Table, Range>(
    db: &'a HeedDb<T::Key, T::Value>,
    tx_ro: &'a heed::RoTxn<'_>,
    range: Range,
) -> Result<impl Iterator<Item = Result<(T::Key, T::Value), RuntimeError>> + 'a, RuntimeError>
where
    Range: RangeBounds<T::Key> + 'a,
{
    Ok(db.range(tx_ro, &range)?.map(|res| Ok(res?)))
}

/// Shared [`DatabaseRo::iter()`].
#[inline]
#[allow(clippy::unnecessary_wraps)]
fn iter<'a, T: Table>(
    db: &'a HeedDb<T::Key, T::Value>,
    tx_ro: &'a heed::RoTxn<'_>,
) -> Result<impl Iterator<Item = Result<(T::Key, T::Value), RuntimeError>> + 'a, RuntimeError> {
    Ok(db.iter(tx_ro)?.map(|res| Ok(res?)))
}

/// Shared [`DatabaseRo::len()`].
#[inline]
fn len<T: Table>(
    db: &HeedDb<T::Key, T::Value>,
    tx_ro: &heed::RoTxn<'_>,
) -> Result<u64, RuntimeError> {
    Ok(db.len(tx_ro)?)
}

/// Shared [`DatabaseRo::first()`].
#[inline]
fn first<T: Table>(
    db: &HeedDb<T::Key, T::Value>,
    tx_ro: &heed::RoTxn<'_>,
) -> Result<(T::Key, T::Value), RuntimeError> {
    todo!()
}

/// Shared [`DatabaseRo::last()`].
#[inline]
fn last<T: Table>(
    db: &HeedDb<T::Key, T::Value>,
    tx_ro: &heed::RoTxn<'_>,
) -> Result<(T::Key, T::Value), RuntimeError> {
    todo!()
}

/// Shared [`DatabaseRo::is_empty()`].
#[inline]
fn is_empty<T: Table>(
    db: &HeedDb<T::Key, T::Value>,
    tx_ro: &heed::RoTxn<'_>,
) -> Result<bool, RuntimeError> {
    todo!()
}

//---------------------------------------------------------------------------------------------------- DatabaseRo Impl
impl<T: Table> DatabaseRo<T> for HeedTableRo<'_, T> {
    #[inline]
    fn get(&self, key: &T::Key) -> Result<T::Value, RuntimeError> {
        get::<T>(&self.db, self.tx_ro, key)
    }

    #[inline]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<(T::Key, T::Value), RuntimeError>> + 'a, RuntimeError>
    where
        Range: RangeBounds<T::Key> + 'a,
    {
        get_range::<T, Range>(&self.db, self.tx_ro, range)
    }

    #[inline]
    fn iter(
        &self,
    ) -> Result<impl Iterator<Item = Result<(T::Key, T::Value), RuntimeError>> + '_, RuntimeError>
    {
        #[allow(clippy::type_complexity)]
        let iter: std::vec::Drain<'_, Result<(T::Key, T::Value), RuntimeError>> = todo!();
        Ok(iter)
    }

    #[inline]
    fn len(&self) -> Result<u64, RuntimeError> {
        todo!()
    }

    #[inline]
    fn first(&self) -> Result<(T::Key, T::Value), RuntimeError> {
        todo!()
    }

    #[inline]
    fn last(&self) -> Result<(T::Key, T::Value), RuntimeError> {
        todo!()
    }

    #[inline]
    fn is_empty(&self) -> Result<bool, RuntimeError> {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRw Impl
impl<T: Table> DatabaseRo<T> for HeedTableRw<'_, '_, T> {
    #[inline]
    fn get(&self, key: &T::Key) -> Result<T::Value, RuntimeError> {
        get::<T>(&self.db, self.tx_rw, key)
    }

    #[inline]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<(T::Key, T::Value), RuntimeError>> + 'a, RuntimeError>
    where
        Range: RangeBounds<T::Key> + 'a,
    {
        get_range::<T, Range>(&self.db, self.tx_rw, range)
    }

    #[inline]
    fn iter(
        &self,
    ) -> Result<impl Iterator<Item = Result<(T::Key, T::Value), RuntimeError>> + '_, RuntimeError>
    {
        #[allow(clippy::type_complexity)]
        let iter: std::vec::Drain<'_, Result<(T::Key, T::Value), RuntimeError>> = todo!();
        Ok(iter)
    }

    #[inline]
    fn len(&self) -> Result<u64, RuntimeError> {
        todo!()
    }

    #[inline]
    fn first(&self) -> Result<(T::Key, T::Value), RuntimeError> {
        todo!()
    }

    #[inline]
    fn last(&self) -> Result<(T::Key, T::Value), RuntimeError> {
        todo!()
    }

    #[inline]
    fn is_empty(&self) -> Result<bool, RuntimeError> {
        todo!()
    }
}

impl<T: Table> DatabaseRw<T> for HeedTableRw<'_, '_, T> {
    #[inline]
    fn put(&mut self, key: &T::Key, value: &T::Value) -> Result<(), RuntimeError> {
        Ok(self.db.put(self.tx_rw, key, value)?)
    }

    #[inline]
    fn delete(&mut self, key: &T::Key) -> Result<(), RuntimeError> {
        self.db.delete(self.tx_rw, key)?;
        Ok(())
    }

    #[inline]
    fn clear(&mut self) -> Result<(), RuntimeError> {
        todo!()
    }

    #[inline]
    fn retain<P>(&mut self) -> Result<(), RuntimeError>
    where
        P: FnMut(T::Key, T::Value) -> bool,
    {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

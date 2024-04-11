//! Implementation of `trait Database` for `heed`.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::{Borrow, Cow},
    cell::RefCell,
    fmt::Debug,
    ops::RangeBounds,
    sync::RwLockReadGuard,
};

use crate::{
    backend::heed::{storable::StorableHeed, types::HeedDb},
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
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
    pub(super) tx_rw: &'tx RefCell<heed::RwTxn<'env>>,
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
    db.first(tx_ro)?.ok_or(RuntimeError::KeyNotFound)
}

/// Shared [`DatabaseRo::last()`].
#[inline]
fn last<T: Table>(
    db: &HeedDb<T::Key, T::Value>,
    tx_ro: &heed::RoTxn<'_>,
) -> Result<(T::Key, T::Value), RuntimeError> {
    db.last(tx_ro)?.ok_or(RuntimeError::KeyNotFound)
}

/// Shared [`DatabaseRo::is_empty()`].
#[inline]
fn is_empty<T: Table>(
    db: &HeedDb<T::Key, T::Value>,
    tx_ro: &heed::RoTxn<'_>,
) -> Result<bool, RuntimeError> {
    Ok(db.is_empty(tx_ro)?)
}

//---------------------------------------------------------------------------------------------------- DatabaseIter Impl
impl<T: Table> DatabaseIter<T> for HeedTableRo<'_, T> {
    #[inline]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<T::Value, RuntimeError>> + 'a, RuntimeError>
    where
        Range: RangeBounds<T::Key> + 'a,
    {
        Ok(self.db.range(self.tx_ro, &range)?.map(|res| Ok(res?.1)))
    }

    #[inline]
    fn iter(
        &self,
    ) -> Result<impl Iterator<Item = Result<(T::Key, T::Value), RuntimeError>> + '_, RuntimeError>
    {
        Ok(self.db.iter(self.tx_ro)?.map(|res| Ok(res?)))
    }

    #[inline]
    fn keys(
        &self,
    ) -> Result<impl Iterator<Item = Result<T::Key, RuntimeError>> + '_, RuntimeError> {
        Ok(self.db.iter(self.tx_ro)?.map(|res| Ok(res?.0)))
    }

    #[inline]
    fn values(
        &self,
    ) -> Result<impl Iterator<Item = Result<T::Value, RuntimeError>> + '_, RuntimeError> {
        Ok(self.db.iter(self.tx_ro)?.map(|res| Ok(res?.1)))
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRo Impl
impl<T: Table> DatabaseRo<T> for HeedTableRo<'_, T> {
    #[inline]
    fn get(&self, key: &T::Key) -> Result<T::Value, RuntimeError> {
        get::<T>(&self.db, self.tx_ro, key)
    }

    #[inline]
    fn len(&self) -> Result<u64, RuntimeError> {
        len::<T>(&self.db, self.tx_ro)
    }

    #[inline]
    fn first(&self) -> Result<(T::Key, T::Value), RuntimeError> {
        first::<T>(&self.db, self.tx_ro)
    }

    #[inline]
    fn last(&self) -> Result<(T::Key, T::Value), RuntimeError> {
        last::<T>(&self.db, self.tx_ro)
    }

    #[inline]
    fn is_empty(&self) -> Result<bool, RuntimeError> {
        is_empty::<T>(&self.db, self.tx_ro)
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRw Impl
impl<T: Table> DatabaseRo<T> for HeedTableRw<'_, '_, T> {
    #[inline]
    fn get(&self, key: &T::Key) -> Result<T::Value, RuntimeError> {
        get::<T>(&self.db, &self.tx_rw.borrow(), key)
    }

    #[inline]
    fn len(&self) -> Result<u64, RuntimeError> {
        len::<T>(&self.db, &self.tx_rw.borrow())
    }

    #[inline]
    fn first(&self) -> Result<(T::Key, T::Value), RuntimeError> {
        first::<T>(&self.db, &self.tx_rw.borrow())
    }

    #[inline]
    fn last(&self) -> Result<(T::Key, T::Value), RuntimeError> {
        last::<T>(&self.db, &self.tx_rw.borrow())
    }

    #[inline]
    fn is_empty(&self) -> Result<bool, RuntimeError> {
        is_empty::<T>(&self.db, &self.tx_rw.borrow())
    }
}

impl<T: Table> DatabaseRw<T> for HeedTableRw<'_, '_, T> {
    #[inline]
    fn put(&mut self, key: &T::Key, value: &T::Value) -> Result<(), RuntimeError> {
        Ok(self.db.put(&mut self.tx_rw.borrow_mut(), key, value)?)
    }

    #[inline]
    fn delete(&mut self, key: &T::Key) -> Result<(), RuntimeError> {
        self.db.delete(&mut self.tx_rw.borrow_mut(), key)?;
        Ok(())
    }

    #[inline]
    fn pop_first(&mut self) -> Result<(T::Key, T::Value), RuntimeError> {
        let tx_rw = &mut self.tx_rw.borrow_mut();

        // Get the first value first...
        let Some(first) = self.db.first(tx_rw)? else {
            return Err(RuntimeError::KeyNotFound);
        };

        // ...then remove it.
        //
        // We use an iterator because we want to semantically
        // remove the _first_ and only the first `(key, value)`.
        // `delete()` removes all keys including duplicates which
        // is slightly different behavior.
        let mut iter = self.db.iter_mut(tx_rw)?;

        // SAFETY:
        // It is undefined behavior to keep a reference of
        // a value from this database while modifying it.
        // We are deleting the value and never accessing
        // the iterator again so this should be safe.
        unsafe {
            iter.del_current()?;
        }

        Ok(first)
    }

    #[inline]
    fn pop_last(&mut self) -> Result<(T::Key, T::Value), RuntimeError> {
        let tx_rw = &mut self.tx_rw.borrow_mut();

        let Some(first) = self.db.last(tx_rw)? else {
            return Err(RuntimeError::KeyNotFound);
        };

        let mut iter = self.db.rev_iter_mut(tx_rw)?;

        // SAFETY:
        // It is undefined behavior to keep a reference of
        // a value from this database while modifying it.
        // We are deleting the value and never accessing
        // the iterator again so this should be safe.
        unsafe {
            iter.del_current()?;
        }

        Ok(first)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

//! Implementation of `trait Database` for `heed`.

//---------------------------------------------------------------------------------------------------- Import
use std::{cell::RefCell, ops::RangeBounds};

use crate::{
    backend::heed::types::HeedDb,
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
    entry::{Entry, OccupiedEntry, VacantEntry},
    error::{DbResult, RuntimeError},
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
) -> DbResult<T::Value> {
    db.get(tx_ro, key)?.ok_or(RuntimeError::KeyNotFound)
}

/// Shared [`DatabaseRo::len()`].
#[inline]
fn len<T: Table>(db: &HeedDb<T::Key, T::Value>, tx_ro: &heed::RoTxn<'_>) -> DbResult<u64> {
    Ok(db.len(tx_ro)?)
}

/// Shared [`DatabaseRo::first()`].
#[inline]
fn first<T: Table>(
    db: &HeedDb<T::Key, T::Value>,
    tx_ro: &heed::RoTxn<'_>,
) -> DbResult<(T::Key, T::Value)> {
    db.first(tx_ro)?.ok_or(RuntimeError::KeyNotFound)
}

/// Shared [`DatabaseRo::last()`].
#[inline]
fn last<T: Table>(
    db: &HeedDb<T::Key, T::Value>,
    tx_ro: &heed::RoTxn<'_>,
) -> DbResult<(T::Key, T::Value)> {
    db.last(tx_ro)?.ok_or(RuntimeError::KeyNotFound)
}

/// Shared [`DatabaseRo::is_empty()`].
#[inline]
fn is_empty<T: Table>(db: &HeedDb<T::Key, T::Value>, tx_ro: &heed::RoTxn<'_>) -> DbResult<bool> {
    Ok(db.is_empty(tx_ro)?)
}

//---------------------------------------------------------------------------------------------------- DatabaseIter Impl
impl<T: Table> DatabaseIter<T> for HeedTableRo<'_, T> {
    #[inline]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
    ) -> DbResult<impl Iterator<Item = DbResult<T::Value>> + 'a>
    where
        Range: RangeBounds<T::Key> + 'a,
    {
        Ok(self.db.range(self.tx_ro, &range)?.map(|res| Ok(res?.1)))
    }

    #[inline]
    fn iter(&self) -> DbResult<impl Iterator<Item = DbResult<(T::Key, T::Value)>> + '_> {
        Ok(self.db.iter(self.tx_ro)?.map(|res| Ok(res?)))
    }

    #[inline]
    fn keys(&self) -> DbResult<impl Iterator<Item = DbResult<T::Key>> + '_> {
        Ok(self.db.iter(self.tx_ro)?.map(|res| Ok(res?.0)))
    }

    #[inline]
    fn values(&self) -> DbResult<impl Iterator<Item = DbResult<T::Value>> + '_> {
        Ok(self.db.iter(self.tx_ro)?.map(|res| Ok(res?.1)))
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRo Impl
// SAFETY: `HeedTableRo: !Send` as it holds a reference to `heed::RoTxn: Send + !Sync`.
unsafe impl<T: Table> DatabaseRo<T> for HeedTableRo<'_, T> {
    #[inline]
    fn get(&self, key: &T::Key) -> DbResult<T::Value> {
        get::<T>(&self.db, self.tx_ro, key)
    }

    #[inline]
    fn len(&self) -> DbResult<u64> {
        len::<T>(&self.db, self.tx_ro)
    }

    #[inline]
    fn first(&self) -> DbResult<(T::Key, T::Value)> {
        first::<T>(&self.db, self.tx_ro)
    }

    #[inline]
    fn last(&self) -> DbResult<(T::Key, T::Value)> {
        last::<T>(&self.db, self.tx_ro)
    }

    #[inline]
    fn is_empty(&self) -> DbResult<bool> {
        is_empty::<T>(&self.db, self.tx_ro)
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRw Impl
// SAFETY: The `Send` bound only applies to `HeedTableRo`.
// `HeedTableRw`'s write transaction is `!Send`.
unsafe impl<T: Table> DatabaseRo<T> for HeedTableRw<'_, '_, T> {
    #[inline]
    fn get(&self, key: &T::Key) -> DbResult<T::Value> {
        get::<T>(&self.db, &self.tx_rw.borrow(), key)
    }

    #[inline]
    fn len(&self) -> DbResult<u64> {
        len::<T>(&self.db, &self.tx_rw.borrow())
    }

    #[inline]
    fn first(&self) -> DbResult<(T::Key, T::Value)> {
        first::<T>(&self.db, &self.tx_rw.borrow())
    }

    #[inline]
    fn last(&self) -> DbResult<(T::Key, T::Value)> {
        last::<T>(&self.db, &self.tx_rw.borrow())
    }

    #[inline]
    fn is_empty(&self) -> DbResult<bool> {
        is_empty::<T>(&self.db, &self.tx_rw.borrow())
    }
}

impl<T: Table> DatabaseRw<T> for HeedTableRw<'_, '_, T> {
    #[inline]
    fn put(&mut self, key: &T::Key, value: &T::Value) -> DbResult<()> {
        Ok(self.db.put(&mut self.tx_rw.borrow_mut(), key, value)?)
    }

    #[inline]
    fn delete(&mut self, key: &T::Key) -> DbResult<()> {
        self.db.delete(&mut self.tx_rw.borrow_mut(), key)?;
        Ok(())
    }

    #[inline]
    fn pop_first(&mut self) -> DbResult<(T::Key, T::Value)> {
        let tx_rw = &mut self.tx_rw.borrow_mut();

        // Get the value first...
        let Some((key, value)) = self.db.first(tx_rw)? else {
            return Err(RuntimeError::KeyNotFound);
        };

        // ...then remove it.
        match self.db.delete(tx_rw, &key) {
            Ok(true) => Ok((key, value)),
            Err(e) => Err(e.into()),
            // We just `get()`'ed the value - it is
            // incorrect for it to suddenly not exist.
            Ok(false) => unreachable!(),
        }
    }

    #[inline]
    fn pop_last(&mut self) -> DbResult<(T::Key, T::Value)> {
        let tx_rw = &mut self.tx_rw.borrow_mut();

        // Get the value first...
        let Some((key, value)) = self.db.last(tx_rw)? else {
            return Err(RuntimeError::KeyNotFound);
        };

        // ...then remove it.
        match self.db.delete(tx_rw, &key) {
            Ok(true) => Ok((key, value)),
            Err(e) => Err(e.into()),
            // We just `get()`'ed the value - it is
            // incorrect for it to suddenly not exist.
            Ok(false) => unreachable!(),
        }
    }

    #[inline]
    fn entry<'a>(&'a mut self, key: &'a T::Key) -> DbResult<Entry<'a, T, Self>> {
        match DatabaseRo::get(self, key) {
            Ok(value) => Ok(Entry::Occupied(OccupiedEntry {
                db: self,
                key,
                value,
            })),
            Err(RuntimeError::KeyNotFound) => Ok(Entry::Vacant(VacantEntry { db: self, key })),
            Err(e) => Err(e),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

//! Implementation of `trait Database` for `heed`.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::{Borrow, Cow},
    cell::RefCell,
    fmt::Debug,
    ops::RangeBounds,
    sync::{Mutex, RwLockReadGuard},
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
    pub(super) tx_ro: Mutex<&'tx heed::RoTxn<'tx>>,
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

#[allow(clippy::non_send_fields_in_send_ty)] // false positive? <https://github.com/rust-lang/rust-clippy/issues/8045>
/// SAFETY: 2 invariants for safety:
///
/// 1. `cuprate_database`'s Cargo.toml enables a feature
/// for `heed` that turns on the `MDB_NOTLS` flag for LMDB.
/// This makes read transactions `Send`, but only if that flag is enabled.
///
/// 2. Our `tx_ro` is wrapped in Mutex, as `&T: Send` only if `T: Sync`.
/// This is what is happening as we have `&TxRw`, not `TxRw`.
/// <https://github.com/Cuprate/cuprate/pull/113#discussion_r1582189108>
///
/// This is required as in `crate::service` we must put our transactions and
/// tables inside `ThreadLocal`'s to use across multiple threads.
unsafe impl<T: Table> Send for HeedTableRo<'_, T> {}

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
        Ok(self
            .db
            .range(&self.tx_ro.lock().unwrap(), &range)?
            .map(|res| Ok(res?.1)))
    }

    #[inline]
    fn iter(
        &self,
    ) -> Result<impl Iterator<Item = Result<(T::Key, T::Value), RuntimeError>> + '_, RuntimeError>
    {
        Ok(self
            .db
            .iter(&self.tx_ro.lock().unwrap())?
            .map(|res| Ok(res?)))
    }

    #[inline]
    fn keys(
        &self,
    ) -> Result<impl Iterator<Item = Result<T::Key, RuntimeError>> + '_, RuntimeError> {
        Ok(self
            .db
            .iter(&self.tx_ro.lock().unwrap())?
            .map(|res| Ok(res?.0)))
    }

    #[inline]
    fn values(
        &self,
    ) -> Result<impl Iterator<Item = Result<T::Value, RuntimeError>> + '_, RuntimeError> {
        Ok(self
            .db
            .iter(&self.tx_ro.lock().unwrap())?
            .map(|res| Ok(res?.1)))
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRo Impl
impl<T: Table> DatabaseRo<T> for HeedTableRo<'_, T> {
    #[inline]
    fn get(&self, key: &T::Key) -> Result<T::Value, RuntimeError> {
        get::<T>(&self.db, &self.tx_ro.lock().unwrap(), key)
    }

    #[inline]
    fn len(&self) -> Result<u64, RuntimeError> {
        len::<T>(&self.db, &self.tx_ro.lock().unwrap())
    }

    #[inline]
    fn first(&self) -> Result<(T::Key, T::Value), RuntimeError> {
        first::<T>(&self.db, &self.tx_ro.lock().unwrap())
    }

    #[inline]
    fn last(&self) -> Result<(T::Key, T::Value), RuntimeError> {
        last::<T>(&self.db, &self.tx_ro.lock().unwrap())
    }

    #[inline]
    fn is_empty(&self) -> Result<bool, RuntimeError> {
        is_empty::<T>(&self.db, &self.tx_ro.lock().unwrap())
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
    fn take(&mut self, key: &T::Key) -> Result<T::Value, RuntimeError> {
        // LMDB/heed does not return the value on deletion.
        // So, fetch it first - then delete.
        let value = get::<T>(&self.db, &self.tx_rw.borrow(), key)?;
        match self.db.delete(&mut self.tx_rw.borrow_mut(), key) {
            Ok(true) => Ok(value),
            Err(e) => Err(e.into()),
            // We just `get()`'ed the value - it is
            // incorrect for it to suddenly not exist.
            Ok(false) => unreachable!(),
        }
    }

    #[inline]
    fn pop_first(&mut self) -> Result<(T::Key, T::Value), RuntimeError> {
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
    fn pop_last(&mut self) -> Result<(T::Key, T::Value), RuntimeError> {
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
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

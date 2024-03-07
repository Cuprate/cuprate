//! Implementation of `trait DatabaseR{o,w}` for `redb`.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::Borrow,
    fmt::Debug,
    ops::{Deref, RangeBounds},
};

use crate::{
    backend::redb::{
        storable::StorableRedb,
        types::{RedbTableRo, RedbTableRw},
    },
    database::{DatabaseRo, DatabaseRw},
    error::RuntimeError,
    storable::Storable,
    table::Table,
};

//---------------------------------------------------------------------------------------------------- Shared functions
// FIXME: we cannot just deref `RedbTableRw -> RedbTableRo` and
// call the functions since the database is held by value, so
// just use these generic functions that both can call instead.

/// TODO
struct AccessGuard<'a, Value>
// This must be done to prevent `Borrow` collisions.
// If `T: Table` was here instead, it causes weird compile errors.
where
    Value: Storable + ?Sized + Debug + 'static,
{
    /// TODO
    access_guard: redb::AccessGuard<'a, StorableRedb<Value>>,
}

impl<Value> Borrow<Value> for AccessGuard<'_, Value>
where
    Value: Storable + ?Sized + Debug + 'static,
{
    #[inline]
    fn borrow(&self) -> &Value {
        self.access_guard.value()
    }
}

/// Shared generic `get()` between `RedbTableR{o,w}`.
#[inline]
fn get<'a, T: Table + 'static>(
    db: &'a impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
    key: &'a T::Key,
) -> Result<impl Borrow<T::Value> + 'a, RuntimeError> {
    match db.get(key) {
        Ok(Some(access_guard)) => Ok(AccessGuard::<T::Value> { access_guard }),
        Ok(None) => Err(RuntimeError::KeyNotFound),
        Err(e) => Err(RuntimeError::from(e)),
    }
}

/// Shared generic `get_range()` between `RedbTableR{o,w}`.
#[inline]
#[allow(clippy::unnecessary_wraps, clippy::trait_duplication_in_bounds)]
fn get_range<'a, T: Table, Range>(
    db: &'a impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
    range: Range,
) -> Result<impl Iterator<Item = Result<impl Borrow<T::Value> + 'a, RuntimeError>>, RuntimeError>
where
    Range: RangeBounds<T::Key> + RangeBounds<&'a T::Key> + 'a,
{
    /// TODO
    struct Iter<'a, K, V>
    where
        K: crate::key::Key + Debug + 'static,
        V: Storable + ?Sized + Debug + 'static,
    {
        /// TODO
        iter: redb::Range<'a, StorableRedb<K>, StorableRedb<V>>,
    }

    // TODO
    impl<'a, K, V> Iterator for Iter<'a, K, V>
    where
        K: crate::key::Key + Debug + 'static,
        V: Storable + ?Sized + Debug + 'static,
    {
        type Item = Result<AccessGuard<'a, V>, RuntimeError>;
        fn next(&mut self) -> Option<Self::Item> {
            // TODO
            self.iter.next().map(|result| match result {
                Ok(kv) => Ok(AccessGuard::<V> { access_guard: kv.1 }),
                Err(e) => Err(RuntimeError::from(e)),
            })
        }
    }

    Ok(Iter::<'a, T::Key, T::Value> {
        iter: db.range::<&'_ T::Key>(range)?,
    })
}

//---------------------------------------------------------------------------------------------------- DatabaseRo
impl<'tx, T: Table + 'static> DatabaseRo<'tx, T> for RedbTableRo<'tx, T::Key, T::Value> {
    #[inline]
    fn get<'a>(&'a self, key: &'a T::Key) -> Result<impl Borrow<T::Value> + 'a, RuntimeError> {
        get::<T>(self, key)
    }

    #[inline]
    #[allow(clippy::unnecessary_wraps, clippy::trait_duplication_in_bounds)]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<impl Borrow<T::Value> + 'a, RuntimeError>>, RuntimeError>
    where
        Range: RangeBounds<T::Key> + RangeBounds<&'a T::Key> + 'a,
    {
        get_range::<T, Range>(self, range)
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRw
impl<'tx, T: Table + 'static> DatabaseRo<'tx, T> for RedbTableRw<'_, 'tx, T::Key, T::Value> {
    #[inline]
    fn get<'a>(&'a self, key: &'a T::Key) -> Result<impl Borrow<T::Value> + 'a, RuntimeError> {
        get::<T>(self, key)
    }

    #[inline]
    #[allow(clippy::unnecessary_wraps, clippy::trait_duplication_in_bounds)]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<impl Borrow<T::Value> + 'a, RuntimeError>>, RuntimeError>
    where
        Range: RangeBounds<T::Key> + RangeBounds<&'a T::Key> + 'a,
    {
        get_range::<T, Range>(self, range)
    }
}

impl<'env, 'tx, T: Table + 'static> DatabaseRw<'env, 'tx, T>
    for RedbTableRw<'env, 'tx, T::Key, T::Value>
{
    // `redb` returns the value after `insert()/remove()`
    // we end with Ok(()) instead.

    #[inline]
    fn put(&mut self, key: &T::Key, value: &T::Value) -> Result<(), RuntimeError> {
        self.insert(key, value)?;
        Ok(())
    }

    #[inline]
    fn delete(&mut self, key: &T::Key) -> Result<(), RuntimeError> {
        self.remove(key)?;
        Ok(())
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

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
struct AccessGuard<'tx, Value>
// This must be done to prevent `Borrow` collisions.
// If `T: Table` was here instead, it causes weird compile errors.
where
    Value: Storable + ?Sized + Debug + 'static,
{
    /// TODO
    access_guard: redb::AccessGuard<'tx, StorableRedb<Value>>,
}

impl<Value> Borrow<Value> for AccessGuard<'_, Value>
where
    Value: Storable + ?Sized + Debug + 'static,
{
    fn borrow(&self) -> &Value {
        self.access_guard.value()
    }
}

/// Shared generic `get()` between `RedbTableR{o,w}`.
#[inline]
fn get<'tx, T: Table + 'static>(
    db: &'tx impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
    key: &'_ T::Key,
) -> Result<impl Borrow<T::Value> + 'tx, RuntimeError> {
    match db.get(key) {
        Ok(Some(access_guard)) => Ok(AccessGuard::<T::Value> { access_guard }),
        Ok(None) => Err(RuntimeError::KeyNotFound),
        Err(e) => Err(RuntimeError::from(e)),
    }
}

/// Shared generic `get_range()` between `RedbTableR{o,w}`.
#[allow(clippy::unnecessary_wraps, clippy::trait_duplication_in_bounds)]
fn get_range<'tx, T: Table, Range>(
    db: &'tx impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
    range: Range,
) -> Result<impl Iterator<Item = Result<impl Borrow<T::Value> + 'tx, RuntimeError>>, RuntimeError>
where
    Range: RangeBounds<T::Key> + RangeBounds<&'tx T::Key> + 'tx,
{
    /// TODO
    struct Iter<'tx, K, V>
    where
        K: crate::key::Key + Debug + 'static,
        V: Storable + ?Sized + Debug + 'static,
    {
        /// TODO
        iter: redb::Range<'tx, StorableRedb<K>, StorableRedb<V>>,
    }

    // TODO
    impl<'tx, K, V> Iterator for Iter<'tx, K, V>
    where
        K: crate::key::Key + Debug + 'static,
        V: Storable + ?Sized + Debug + 'static,
    {
        type Item = Result<AccessGuard<'tx, V>, RuntimeError>;
        fn next(&mut self) -> Option<Self::Item> {
            // TODO
            self.iter.next().map(|result| match result {
                Ok(kv) => Ok(AccessGuard::<V> { access_guard: kv.1 }),
                Err(e) => Err(RuntimeError::from(e)),
            })
        }
    }

    Ok(Iter::<'tx, T::Key, T::Value> {
        iter: db.range::<&'_ T::Key>(range)?,
    })
}

//---------------------------------------------------------------------------------------------------- DatabaseRo
impl<'tx, T: Table + 'static> DatabaseRo<'tx, T> for RedbTableRo<'tx, T::Key, T::Value> {
    fn get(&'tx self, key: &T::Key) -> Result<impl Borrow<T::Value> + 'tx, RuntimeError> {
        get::<T>(self, key)
    }

    #[allow(clippy::unnecessary_wraps, clippy::trait_duplication_in_bounds)]
    fn get_range<Range>(
        &'tx self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<impl Borrow<T::Value> + 'tx, RuntimeError>>, RuntimeError>
    where
        Range: RangeBounds<T::Key> + RangeBounds<&'tx T::Key> + 'tx,
    {
        get_range::<T, Range>(self, range)
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRw
impl<'tx, T: Table + 'static> DatabaseRo<'tx, T> for RedbTableRw<'tx, 'tx, T::Key, T::Value> {
    fn get(&'tx self, key: &T::Key) -> Result<impl Borrow<T::Value> + 'tx, RuntimeError> {
        get::<T>(self, key)
    }

    #[allow(clippy::unnecessary_wraps, clippy::trait_duplication_in_bounds)]
    fn get_range<Range>(
        &'tx self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<impl Borrow<T::Value> + 'tx, RuntimeError>>, RuntimeError>
    where
        Range: RangeBounds<T::Key> + RangeBounds<&'tx T::Key> + 'tx,
    {
        get_range::<T, Range>(self, range)
    }
}

impl<'tx, T: Table + 'static> DatabaseRw<'tx, T> for RedbTableRw<'tx, 'tx, T::Key, T::Value> {
    fn put(&mut self, key: &T::Key, value: &T::Value) -> Result<(), RuntimeError> {
        self.insert(key, value)?;
        Ok(())
    }

    fn delete(&mut self, key: &T::Key) -> Result<(), RuntimeError> {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

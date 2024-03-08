//! Implementation of `trait DatabaseR{o,w}` for `redb`.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::{Borrow, Cow},
    fmt::Debug,
    ops::{Deref, RangeBounds},
};

use crate::{
    backend::redb::{
        storable::{StorableRedbKey, StorableRedbValue},
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

// /// TODO
// struct AccessGuard<'a, Value>
// // This must be done to prevent `Borrow` collisions.
// // If `T: Table` was here instead, it causes weird compile errors.
// where
//     Value: Storable + Clone + ?Sized + Debug + 'static,
// {
//     /// TODO
//     access_guard: redb::AccessGuard<'a, StorableRedbValue<Value>>,
// }

// impl<Value> Borrow<Value> for AccessGuard<'_, Value>
// where
//     Value: Storable + Clone + ?Sized + Debug + 'static,
// {
//     #[inline]
//     fn borrow(&self) -> &Value {
//         self.access_guard.value()
//     }
// }

// TODO: document that `Cow` essentially acts as our
// `AccessGuard` now, and that we know all values are
// owned to begin with, so `.into_owned()` is cheap.
//
// Invariant should be upheld (panic on unowned?).

/// Shared generic `get()` between `RedbTableR{o,w}`.
#[inline]
fn get<'a, T: Table + 'static>(
    db: &'a impl redb::ReadableTable<StorableRedbKey<T::Key>, StorableRedbValue<T::Value>>,
    key: &'a T::Key,
) -> Result<impl Borrow<T::Value> + 'a, RuntimeError> {
    match db.get(key) {
        Ok(Some(access_guard)) => Ok(Cow::Owned(access_guard.value().into_owned())),
        Ok(None) => Err(RuntimeError::KeyNotFound),
        Err(e) => Err(RuntimeError::from(e)),
    }
}

/// Shared generic `get_range()` between `RedbTableR{o,w}`.
#[inline]
#[allow(clippy::unnecessary_wraps, clippy::trait_duplication_in_bounds)]
fn get_range<'a, T: Table, Range>(
    db: &'a impl redb::ReadableTable<StorableRedbKey<T::Key>, StorableRedbValue<T::Value>>,
    range: Range,
) -> Result<impl Iterator<Item = Result<impl Borrow<T::Value> + 'a, RuntimeError>>, RuntimeError>
where
    Range: RangeBounds<T::Key> + RangeBounds<&'a T::Key> + 'a,
{
    /// TODO
    struct Iter<'a, K, V>
    where
        K: crate::key::Key + Clone + Debug + 'static,
        V: Storable + Clone + ?Sized + Debug + 'static,
    {
        /// TODO
        iter: redb::Range<'a, StorableRedbKey<K>, StorableRedbValue<V>>,
    }

    // TODO
    impl<'a, K, V> Iterator for Iter<'a, K, V>
    where
        K: crate::key::Key + Clone + Debug + 'static,
        V: Storable + Clone + ?Sized + Debug + 'static,
    {
        type Item = Result<Cow<'a, V>, RuntimeError>;
        fn next(&mut self) -> Option<Self::Item> {
            // TODO
            self.iter.next().map(|result| match result {
                Ok(kv) => Ok(Cow::Owned(kv.1.value().into_owned())),
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
        self.insert(key, Cow::Borrowed(value))?;
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

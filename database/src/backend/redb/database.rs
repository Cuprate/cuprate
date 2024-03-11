//! Implementation of `trait DatabaseR{o,w}` for `redb`.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::{Borrow, Cow},
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

/// Shared generic `get()` between `RedbTableR{o,w}`.
#[inline]
fn get<'a, 'b, T: Table + 'static>(
    db: &'a impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
    key: &'a T::Key,
    value_guard: &'b mut Option<redb::AccessGuard<'a, StorableRedb<T::Value>>>,
) -> Result<Cow<'b, T::Value>, RuntimeError> {
    match db.get(Cow::Borrowed(key)) {
        Ok(Some(cow)) => {
            *value_guard = Some(cow);
            Ok(value_guard.as_ref().unwrap().value())
        }
        Ok(None) => Err(RuntimeError::KeyNotFound),
        Err(e) => Err(RuntimeError::from(e)),
    }
}

/// Shared generic `get_range()` between `RedbTableR{o,w}`.
#[inline]
#[allow(
    clippy::unnecessary_wraps,
    clippy::trait_duplication_in_bounds,
    clippy::needless_pass_by_value
)]
fn get_range<'a, T: Table, Range>(
    db: &'a impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
    range: Range,
) -> Result<impl Iterator<Item = Result<Cow<'a, T::Value>, RuntimeError>>, RuntimeError>
where
    Range: RangeBounds<Cow<'a, T::Key>> + 'a,
{
    /// TODO
    struct Iter<'a, K, V>
    where
        K: crate::key::Key + 'static,
        V: Storable + ?Sized + 'static,
    {
        /// TODO
        iter: redb::Range<'a, StorableRedb<K>, StorableRedb<V>>,
    }

    // TODO
    impl<'a, K, V> Iterator for Iter<'a, K, V>
    where
        K: crate::key::Key + 'static,
        V: Storable + ?Sized + 'static,
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
        iter: db.range::<Cow<'a, T::Key>>(range)?,
    })
}

//---------------------------------------------------------------------------------------------------- DatabaseRo
impl<'tx, T: Table + 'static> DatabaseRo<'tx, T> for RedbTableRo<'tx, T::Key, T::Value> {
    type ValueGuard<'a> = redb::AccessGuard<'a, StorableRedb<T::Value>>
        where
            Self: 'a;

    #[inline]
    fn get<'a, 'b>(
        &'a self,
        key: &'a T::Key,
        value_guard: &'b mut Option<Self::ValueGuard<'a>>,
    ) -> Result<Cow<'b, T::Value>, RuntimeError> {
        get::<T>(self, key, value_guard)
    }

    #[inline]
    #[allow(clippy::unnecessary_wraps, clippy::trait_duplication_in_bounds)]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<Cow<'a, T::Value>, RuntimeError>>, RuntimeError>
    where
        Range: RangeBounds<Cow<'a, T::Key>> + 'a,
    {
        get_range::<T, Range>(self, range)
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRw
impl<'tx, T: Table + 'static> DatabaseRo<'tx, T> for RedbTableRw<'_, 'tx, T::Key, T::Value> {
    type ValueGuard<'a> = redb::AccessGuard<'a, StorableRedb<T::Value>>
        where
            Self: 'a;

    #[inline]
    fn get<'a, 'b>(
        &'a self,
        key: &'a T::Key,
        value_guard: &'b mut Option<Self::ValueGuard<'a>>,
    ) -> Result<Cow<'b, T::Value>, RuntimeError> {
        get::<T>(self, key, value_guard)
    }

    #[inline]
    #[allow(clippy::unnecessary_wraps, clippy::trait_duplication_in_bounds)]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<Cow<'a, T::Value>, RuntimeError>>, RuntimeError>
    where
        Range: RangeBounds<Cow<'a, T::Key>> + 'a,
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
        self.insert(Cow::Borrowed(key), Cow::Borrowed(value))?;
        Ok(())
    }

    #[inline]
    fn delete(&mut self, key: &T::Key) -> Result<(), RuntimeError> {
        self.remove(Cow::Borrowed(key))?;
        Ok(())
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

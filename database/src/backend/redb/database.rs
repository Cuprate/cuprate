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
    value_guard::ValueGuard,
};

//---------------------------------------------------------------------------------------------------- Shared functions
// FIXME: we cannot just deref `RedbTableRw -> RedbTableRo` and
// call the functions since the database is held by value, so
// just use these generic functions that both can call instead.

/// Shared generic `get()` between `RedbTableR{o,w}`.
#[inline]
fn get<'a, T: Table + 'static>(
    db: &'a impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
    key: &'a T::Key,
) -> Result<impl ValueGuard<T::Value> + 'a, RuntimeError> {
    match db.get(Cow::Borrowed(key)) {
        Ok(Some(redb_guard)) => Ok(redb_guard),
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
) -> Result<
    impl Iterator<Item = Result<redb::AccessGuard<'a, StorableRedb<T::Value>>, RuntimeError>> + 'a,
    RuntimeError,
>
where
    Range: RangeBounds<Cow<'a, T::Key>> + 'a,
{
    Ok(db.range(range)?.map(|result| {
        let (_key, value_guard) = result?;
        Ok(value_guard)
    }))
}

//---------------------------------------------------------------------------------------------------- DatabaseRo
impl<'tx, T: Table + 'static> DatabaseRo<'tx, T> for RedbTableRo<'tx, T::Key, T::Value> {
    #[inline]
    fn get<'a>(&'a self, key: &'a T::Key) -> Result<impl ValueGuard<T::Value> + 'a, RuntimeError> {
        get::<T>(self, key)
    }

    #[inline]
    #[allow(clippy::unnecessary_wraps, clippy::trait_duplication_in_bounds)]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
        // value_guard: &'a mut Option<redb::AccessGuard<'a, StorableRedb<T::Value>>>,
    ) -> Result<
        impl Iterator<Item = Result<impl ValueGuard<T::Value>, RuntimeError>> + 'a,
        RuntimeError,
    >
    // ) -> Result<impl Iterator<Item = Result<Self::ValueGuard<'a>, RuntimeError>>, RuntimeError>
    where
        Range: RangeBounds<Cow<'a, T::Key>> + 'a,
    {
        get_range::<T, Range>(self, range)
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRw
impl<'tx, T: Table + 'static> DatabaseRo<'tx, T> for RedbTableRw<'_, 'tx, T::Key, T::Value> {
    #[inline]
    fn get<'a>(&'a self, key: &'a T::Key) -> Result<impl ValueGuard<T::Value> + 'a, RuntimeError> {
        get::<T>(self, key)
    }

    #[inline]
    #[allow(clippy::unnecessary_wraps, clippy::trait_duplication_in_bounds)]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
        // value_guard: &'b mut Option<Self::ValueGuard<'a>>,
    ) -> Result<
        impl Iterator<Item = Result<impl ValueGuard<T::Value>, RuntimeError>> + 'a,
        RuntimeError,
    >
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

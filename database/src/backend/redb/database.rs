//! Implementation of `trait DatabaseR{o,w}` for `redb`.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::{Borrow, Cow},
    fmt::Debug,
    marker::PhantomData,
    ops::{Bound, Deref, RangeBounds},
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
fn get<T: Table + 'static>(
    db: &impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
    key: &T::Key,
) -> Result<T::Value, RuntimeError> {
    Ok(db.get(key)?.ok_or(RuntimeError::KeyNotFound)?.value())
}

/// Shared generic `get_range()` between `RedbTableR{o,w}`.
#[inline]
fn get_range<'a, T: Table, Range>(
    db: &'a impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
    range: Range,
) -> Result<impl Iterator<Item = Result<T::Value, RuntimeError>> + 'a, RuntimeError>
where
    Range: RangeBounds<T::Key> + 'a,
{
    Ok(db.range(range)?.map(|result| {
        let (_key, value_guard) = result?;
        Ok(value_guard.value())
    }))
}

//---------------------------------------------------------------------------------------------------- DatabaseRo
impl<T: Table + 'static> DatabaseRo<T> for RedbTableRo<T::Key, T::Value> {
    #[inline]
    fn get(&self, key: &T::Key) -> Result<T::Value, RuntimeError> {
        get::<T>(self, key)
    }

    #[inline]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<T::Value, RuntimeError>> + 'a, RuntimeError>
    where
        Range: RangeBounds<T::Key> + 'a,
    {
        get_range::<T, Range>(self, range)
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRw
impl<T: Table + 'static> DatabaseRo<T> for RedbTableRw<'_, T::Key, T::Value> {
    #[inline]
    fn get(&self, key: &T::Key) -> Result<T::Value, RuntimeError> {
        get::<T>(self, key)
    }

    #[inline]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<T::Value, RuntimeError>> + 'a, RuntimeError>
    where
        Range: RangeBounds<T::Key> + 'a,
    {
        get_range::<T, Range>(self, range)
    }
}

impl<'tx, T: Table + 'static> DatabaseRw<'_, 'tx, T> for RedbTableRw<'tx, T::Key, T::Value> {
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

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

/// Shared [`DatabaseRo::get()`].
#[inline]
fn get<T: Table + 'static>(
    db: &impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
    key: &T::Key,
) -> Result<T::Value, RuntimeError> {
    Ok(db.get(key)?.ok_or(RuntimeError::KeyNotFound)?.value())
}

/// Shared [`DatabaseRo::get_range()`].
#[inline]
fn get_range<'a, T: Table, Range>(
    db: &'a impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
    range: Range,
) -> Result<impl Iterator<Item = Result<(T::Key, T::Value), RuntimeError>> + 'a, RuntimeError>
where
    Range: RangeBounds<T::Key> + 'a,
{
    Ok(db.range(range)?.map(|result| {
        let (key, value) = result?;
        Ok((key.value(), value.value()))
    }))
}

/// Shared [`DatabaseRo::iter()`].
#[inline]
#[allow(clippy::unnecessary_wraps)]
fn iter<T: Table>(
    db: &impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
) -> Result<impl Iterator<Item = Result<(T::Key, T::Value), RuntimeError>> + '_, RuntimeError> {
    Ok(db.iter()?.map(|result| {
        let (key, value) = result?;
        Ok((key.value(), value.value()))
    }))
}

/// Shared [`DatabaseRo::len()`].
#[inline]
fn len<T: Table>(
    db: &impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
) -> Result<u64, RuntimeError> {
    Ok(db.len()?)
}

/// Shared [`DatabaseRo::first()`].
#[inline]
fn first<T: Table>(
    db: &impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
) -> Result<(T::Key, T::Value), RuntimeError> {
    let (key, value) = db.first()?.ok_or(RuntimeError::KeyNotFound)?;
    Ok((key.value(), value.value()))
}

/// Shared [`DatabaseRo::last()`].
#[inline]
fn last<T: Table>(
    db: &impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
) -> Result<(T::Key, T::Value), RuntimeError> {
    let (key, value) = db.last()?.ok_or(RuntimeError::KeyNotFound)?;
    Ok((key.value(), value.value()))
}

/// Shared [`DatabaseRo::is_empty()`].
#[inline]
fn is_empty<T: Table>(
    db: &impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
) -> Result<bool, RuntimeError> {
    Ok(db.is_empty()?)
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
    ) -> Result<impl Iterator<Item = Result<(T::Key, T::Value), RuntimeError>> + 'a, RuntimeError>
    where
        Range: RangeBounds<T::Key> + 'a,
    {
        get_range::<T, Range>(self, range)
    }

    #[inline]
    fn iter(
        &self,
    ) -> Result<impl Iterator<Item = Result<(T::Key, T::Value), RuntimeError>> + '_, RuntimeError>
    {
        iter::<T>(self)
    }

    #[inline]
    fn len(&self) -> Result<u64, RuntimeError> {
        len::<T>(self)
    }

    #[inline]
    fn first(&self) -> Result<(T::Key, T::Value), RuntimeError> {
        first::<T>(self)
    }

    #[inline]
    fn last(&self) -> Result<(T::Key, T::Value), RuntimeError> {
        last::<T>(self)
    }

    #[inline]
    fn is_empty(&self) -> Result<bool, RuntimeError> {
        is_empty::<T>(self)
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
    ) -> Result<impl Iterator<Item = Result<(T::Key, T::Value), RuntimeError>> + 'a, RuntimeError>
    where
        Range: RangeBounds<T::Key> + 'a,
    {
        get_range::<T, Range>(self, range)
    }

    #[inline]
    fn iter(
        &self,
    ) -> Result<impl Iterator<Item = Result<(T::Key, T::Value), RuntimeError>> + '_, RuntimeError>
    {
        iter::<T>(self)
    }

    #[inline]
    fn len(&self) -> Result<u64, RuntimeError> {
        len::<T>(self)
    }

    #[inline]
    fn first(&self) -> Result<(T::Key, T::Value), RuntimeError> {
        first::<T>(self)
    }

    #[inline]
    fn last(&self) -> Result<(T::Key, T::Value), RuntimeError> {
        last::<T>(self)
    }

    #[inline]
    fn is_empty(&self) -> Result<bool, RuntimeError> {
        is_empty::<T>(self)
    }
}

impl<T: Table + 'static> DatabaseRw<T> for RedbTableRw<'_, T::Key, T::Value> {
    // `redb` returns the value after function calls so we end with Ok(()) instead.

    #[inline]
    fn put(&mut self, key: &T::Key, value: &T::Value) -> Result<(), RuntimeError> {
        redb::Table::insert(self, key, value)?;
        Ok(())
    }

    #[inline]
    fn delete(&mut self, key: &T::Key) -> Result<(), RuntimeError> {
        redb::Table::remove(self, key)?;
        Ok(())
    }

    #[inline]
    fn clear(&mut self) -> Result<(), RuntimeError> {
        // FIXME:
        // Deleting and re-creating the table outright
        // would be faster, although we don't have direct
        // access to the write transaction, which implements
        // this functionality in `redb::WriteTransaction::delete_table`.

        // Pop the last `(key, value)` until there are none.
        while self.pop_last()?.is_some() {}

        Ok(())
    }

    #[inline]
    fn retain<P>(&mut self, predicate: P) -> Result<(), RuntimeError>
    where
        P: FnMut(T::Key, T::Value) -> bool,
    {
        redb::Table::retain(self, predicate)?;
        Ok(())
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

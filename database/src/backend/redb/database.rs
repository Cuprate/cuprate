//! Implementation of `trait DatabaseR{o,w}` for `redb`.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::{Borrow, Cow},
    fmt::Debug,
    marker::PhantomData,
    ops::{Bound, Deref, RangeBounds},
};

use redb::ReadableTable;

use crate::{
    backend::redb::{
        storable::StorableRedb,
        types::{RedbTableRo, RedbTableRw},
    },
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
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

//---------------------------------------------------------------------------------------------------- DatabaseIter
impl<T: Table + 'static> DatabaseIter<T> for RedbTableRo<T::Key, T::Value> {
    #[inline]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<T::Value, RuntimeError>> + 'a, RuntimeError>
    where
        Range: RangeBounds<T::Key> + 'a,
    {
        Ok(ReadableTable::range(self, range)?.map(|result| {
            let (_key, value) = result?;
            Ok(value.value())
        }))
    }

    #[inline]
    fn iter(
        &self,
    ) -> Result<impl Iterator<Item = Result<(T::Key, T::Value), RuntimeError>> + '_, RuntimeError>
    {
        Ok(ReadableTable::iter(self)?.map(|result| {
            let (key, value) = result?;
            Ok((key.value(), value.value()))
        }))
    }

    #[inline]
    fn keys(
        &self,
    ) -> Result<impl Iterator<Item = Result<T::Key, RuntimeError>> + '_, RuntimeError> {
        Ok(ReadableTable::iter(self)?.map(|result| {
            let (key, _value) = result?;
            Ok(key.value())
        }))
    }

    #[inline]
    fn values(
        &self,
    ) -> Result<impl Iterator<Item = Result<T::Value, RuntimeError>> + '_, RuntimeError> {
        Ok(ReadableTable::iter(self)?.map(|result| {
            let (_key, value) = result?;
            Ok(value.value())
        }))
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRo
impl<T: Table + 'static> DatabaseRo<T> for RedbTableRo<T::Key, T::Value> {
    #[inline]
    fn get(&self, key: &T::Key) -> Result<T::Value, RuntimeError> {
        get::<T>(self, key)
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
    fn pop_first(&mut self) -> Result<(T::Key, T::Value), RuntimeError> {
        let (key, value) = redb::Table::pop_first(self)?.ok_or(RuntimeError::KeyNotFound)?;
        Ok((key.value(), value.value()))
    }

    #[inline]
    fn pop_last(&mut self) -> Result<(T::Key, T::Value), RuntimeError> {
        let (key, value) = redb::Table::pop_last(self)?.ok_or(RuntimeError::KeyNotFound)?;
        Ok((key.value(), value.value()))
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

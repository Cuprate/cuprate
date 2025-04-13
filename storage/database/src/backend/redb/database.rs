//! Implementation of `trait DatabaseR{o,w}` for `redb`.

//---------------------------------------------------------------------------------------------------- Import
use redb::ReadableTable;

use crate::{
    backend::redb::{
        storable::StorableRedb,
        types::{RedbTableRo, RedbTableRw},
    },
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
    entry::{Entry, OccupiedEntry, VacantEntry},
    error::{DbResult, RuntimeError},
    table::Table,
};

//---------------------------------------------------------------------------------------------------- Shared functions
// FIXME: we cannot just deref `RedbTableRw -> RedbTableRo` and
// call the functions since the database is held by value, so
// just use these generic functions that both can call instead.

/// Shared [`DatabaseRo::get()`].
#[inline]
fn get<T: Table + 'static>(
    db: &impl ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
    key: &T::Key,
) -> DbResult<T::Value> {
    Ok(db.get(key)?.ok_or(RuntimeError::KeyNotFound)?.value())
}

/// Shared [`DatabaseRo::len()`].
#[inline]
fn len<T: Table>(
    db: &impl ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
) -> DbResult<u64> {
    Ok(db.len()?)
}

/// Shared [`DatabaseRo::first()`].
#[inline]
fn first<T: Table>(
    db: &impl ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
) -> DbResult<(T::Key, T::Value)> {
    let (key, value) = db.first()?.ok_or(RuntimeError::KeyNotFound)?;
    Ok((key.value(), value.value()))
}

/// Shared [`DatabaseRo::last()`].
#[inline]
fn last<T: Table>(
    db: &impl ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
) -> DbResult<(T::Key, T::Value)> {
    let (key, value) = db.last()?.ok_or(RuntimeError::KeyNotFound)?;
    Ok((key.value(), value.value()))
}

/// Shared [`DatabaseRo::is_empty()`].
#[inline]
fn is_empty<T: Table>(
    db: &impl ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
) -> DbResult<bool> {
    Ok(db.is_empty()?)
}

//---------------------------------------------------------------------------------------------------- DatabaseIter
impl<T: Table + 'static> DatabaseIter<T> for RedbTableRo<T::Key, T::Value> {
    /*
    #[inline]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
    ) -> DbResult<impl Iterator<Item = DbResult<T::Value>> + 'a>
    where
        Range: RangeBounds<T::Key> + 'a,
    {
        Ok(ReadableTable::range(self, range)?.map(|result| {
            let (_key, value) = result?;
            Ok(value.value())
        }))
    }

     */

    #[inline]
    fn iter(&self) -> DbResult<impl Iterator<Item = DbResult<(T::Key, T::Value)>> + '_> {
        Ok(ReadableTable::iter(self)?.map(|result| {
            let (key, value) = result?;
            Ok((key.value(), value.value()))
        }))
    }

    #[inline]
    fn keys(&self) -> DbResult<impl Iterator<Item = DbResult<T::Key>> + '_> {
        Ok(ReadableTable::iter(self)?.map(|result| {
            let (key, _value) = result?;
            Ok(key.value())
        }))
    }

    #[inline]
    fn values(&self) -> DbResult<impl Iterator<Item = DbResult<T::Value>> + '_> {
        Ok(ReadableTable::iter(self)?.map(|result| {
            let (_key, value) = result?;
            Ok(value.value())
        }))
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRo
// SAFETY: Both `redb`'s transaction and table types are `Send + Sync`.
unsafe impl<T: Table + 'static> DatabaseRo<T> for RedbTableRo<T::Key, T::Value> {
    #[inline]
    fn get(&self, key: &T::Key) -> DbResult<T::Value> {
        get::<T>(self, key)
    }

    #[inline]
    fn len(&self) -> DbResult<u64> {
        len::<T>(self)
    }

    #[inline]
    fn first(&self) -> DbResult<(T::Key, T::Value)> {
        first::<T>(self)
    }

    #[inline]
    fn last(&self) -> DbResult<(T::Key, T::Value)> {
        last::<T>(self)
    }

    #[inline]
    fn is_empty(&self) -> DbResult<bool> {
        is_empty::<T>(self)
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRw
// SAFETY: Both `redb`'s transaction and table types are `Send + Sync`.
unsafe impl<T: Table + 'static> DatabaseRo<T> for RedbTableRw<'_, T::Key, T::Value> {
    #[inline]
    fn get(&self, key: &T::Key) -> DbResult<T::Value> {
        get::<T>(self, key)
    }

    #[inline]
    fn len(&self) -> DbResult<u64> {
        len::<T>(self)
    }

    #[inline]
    fn first(&self) -> DbResult<(T::Key, T::Value)> {
        first::<T>(self)
    }

    #[inline]
    fn last(&self) -> DbResult<(T::Key, T::Value)> {
        last::<T>(self)
    }

    #[inline]
    fn is_empty(&self) -> DbResult<bool> {
        is_empty::<T>(self)
    }
}

impl<T: Table> DatabaseRw<T> for RedbTableRw<'_, T::Key, T::Value> {
    // `redb` returns the value after function calls so we end with Ok(()) instead.

    #[inline]
    fn put(&mut self, key: &T::Key, value: &T::Value) -> DbResult<()> {
        redb::Table::insert(self, key, value)?;
        Ok(())
    }

    #[inline]
    fn delete(&mut self, key: &T::Key) -> DbResult<()> {
        redb::Table::remove(self, key)?;
        Ok(())
    }

    #[inline]
    fn take(&mut self, key: &T::Key) -> DbResult<T::Value> {
        if let Some(value) = redb::Table::remove(self, key)? {
            Ok(value.value())
        } else {
            Err(RuntimeError::KeyNotFound)
        }
    }

    #[inline]
    fn pop_first(&mut self) -> DbResult<(T::Key, T::Value)> {
        let (key, value) = redb::Table::pop_first(self)?.ok_or(RuntimeError::KeyNotFound)?;
        Ok((key.value(), value.value()))
    }

    #[inline]
    fn pop_last(&mut self) -> DbResult<(T::Key, T::Value)> {
        let (key, value) = redb::Table::pop_last(self)?.ok_or(RuntimeError::KeyNotFound)?;
        Ok((key.value(), value.value()))
    }

    #[inline]
    fn entry<'a>(&'a mut self, key: &'a T::Key) -> DbResult<Entry<'a, T, Self>> {
        match get::<T>(self, key) {
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

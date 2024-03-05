//! Implementation of `trait DatabaseR{o,w}` for `redb`.

//---------------------------------------------------------------------------------------------------- Import
use std::ops::Deref;

use crate::{
    backend::redb::{
        storable::StorableRedb,
        types::{RedbTableRo, RedbTableRw},
    },
    database::{DatabaseRo, DatabaseRw},
    error::RuntimeError,
    table::Table,
    value_guard::ValueGuard,
};

//---------------------------------------------------------------------------------------------------- Shared functions
// FIXME: we cannot just deref `RedbTableRw -> RedbTableRo` and
// call the functions since the database is held by value, so
// just use these generic functions that both can call instead.

/// TODO
struct AccessGuard<'tx, T: Table> {
    /// TODO
    access_guard: redb::AccessGuard<'tx, StorableRedb<T::Value>>,
}

impl<T: Table> ValueGuard<'_, T::Value> for AccessGuard<'_, T> {
    fn value(&'_ self) -> &'_ T::Value {
        self.access_guard.value()
    }
}

/// Shared generic `get()` between `RedbTableR{o,w}`.
#[inline]
fn get<'tx, T: Table + 'static>(
    db: &'tx impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
    key: &'_ T::Key,
) -> Result<impl ValueGuard<'tx, T::Value>, RuntimeError> {
    match db.get(key) {
        Ok(Some(access_guard)) => Ok(AccessGuard::<T> { access_guard }),
        Ok(None) => Err(RuntimeError::KeyNotFound),
        Err(e) => Err(RuntimeError::from(e)),
    }
}

/// Shared generic `get_range()` between `RedbTableR{o,w}`.
fn get_range<'tx, T: Table, R: std::ops::RangeBounds<T::Key>>(
    db: &'_ impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
    range: R,
) -> impl Iterator<Item = Result<impl ValueGuard<'tx, T::Value>, RuntimeError>> {
    /// TODO
    struct Iter<'iter, T: Table> {
        /// TODO
        iter: redb::Range<'iter, StorableRedb<T::Key>, StorableRedb<T::Value>>,
    }

    // TODO
    impl<'iter, T: Table> Iterator for Iter<'iter, T> {
        type Item = Result<AccessGuard<'iter, T>, RuntimeError>;
        fn next(&mut self) -> Option<Self::Item> {
            // TODO
            self.iter.next().map(|result| {
                result
                    .map(|value| AccessGuard::<T> {
                        access_guard: value.1,
                    })
                    .map_err(RuntimeError::from)
            })
        }
    }

    Iter::<'_, T> {
        // iter: db.range(range)?,
        iter: todo!(),
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRo
impl<'tx, T: Table + 'static> DatabaseRo<'tx, T> for RedbTableRo<'tx, T::Key, T::Value> {
    fn get(&'tx self, key: &T::Key) -> Result<impl ValueGuard<'tx, T::Value>, RuntimeError> {
        get::<T>(self, key)
    }

    fn get_range<R: std::ops::RangeBounds<T::Key>>(
        &self,
        range: R,
    ) -> impl Iterator<Item = Result<impl ValueGuard<'tx, T::Value>, RuntimeError>> {
        get_range::<T, R>(self, range)
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRw
impl<'tx, T: Table + 'static> DatabaseRo<'tx, T> for RedbTableRw<'tx, 'tx, T::Key, T::Value> {
    fn get(&'tx self, key: &T::Key) -> Result<impl ValueGuard<'tx, T::Value>, RuntimeError> {
        get::<T>(self, key)
    }

    fn get_range<R: std::ops::RangeBounds<T::Key>>(
        &self,
        range: R,
    ) -> impl Iterator<Item = Result<impl ValueGuard<'tx, T::Value>, RuntimeError>> {
        get_range::<T, R>(self, range)
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

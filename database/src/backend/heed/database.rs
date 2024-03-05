//! Implementation of `trait Database` for `heed`.

//---------------------------------------------------------------------------------------------------- Import
use std::{borrow::Borrow, ops::RangeBounds};

use crate::{
    backend::heed::{storable::StorableHeed, types::HeedDb},
    database::{DatabaseRo, DatabaseRw},
    error::RuntimeError,
    table::Table,
};

//---------------------------------------------------------------------------------------------------- Heed Database Wrappers
// Q. Why does `HeedTableR{o,w}` exist?
// A. These wrapper types combine `heed`'s database/table
// types with its transaction types. It exists to match
// `redb`, which has this behavior built-in.
//
// `redb` forces us to abstract read/write semantics
// at the _opened table_ level, so, we must match that in `heed`,
// which abstracts it at the transaction level.
//
// We must also maintain the ability for
// write operations to also read, aka, `Rw`.

/// An opened read-only database associated with a transaction.
///
/// Matches `redb::ReadOnlyTable`.
pub(super) struct HeedTableRo<'env, T: Table> {
    /// An already opened database table.
    pub(super) db: HeedDb<T::Key, T::Value>,
    /// The associated read-only transaction that opened this table.
    pub(super) tx_ro: &'env heed::RoTxn<'env>,
}

/// An opened read/write database associated with a transaction.
///
/// Matches `redb::Table` (read & write).
pub(super) struct HeedTableRw<'env, T: Table> {
    /// TODO
    pub(super) db: HeedDb<T::Key, T::Value>,
    /// The associated read/write transaction that opened this table.
    pub(super) tx_rw: &'env mut heed::RwTxn<'env>,
}

//---------------------------------------------------------------------------------------------------- Shared functions
// FIXME: we cannot just deref `HeedTableRw -> HeedTableRo` and
// call the functions since the database is held by value, so
// just use these generic functions that both can call instead.

/// Shared generic `get()` between `HeedTableR{o,w}`.
#[inline]
fn get<'tx, T: Table>(
    db: &'_ HeedDb<T::Key, T::Value>,
    tx_ro: &'tx heed::RoTxn<'tx>,
    key: &'_ T::Key,
) -> Result<impl Borrow<T::Value> + 'tx, RuntimeError> {
    match db.get(tx_ro, key) {
        Ok(Some(value)) => Ok(value),
        Ok(None) => Err(RuntimeError::KeyNotFound),
        Err(e) => Err(e.into()),
    }
}

/// Shared generic `get_range()` between `HeedTableR{o,w}`.
#[allow(clippy::needless_pass_by_value, clippy::trait_duplication_in_bounds)]
fn get_range<'tx, T: Table, Range>(
    db: &'_ HeedDb<T::Key, T::Value>,
    tx_ro: &'tx heed::RoTxn<'tx>,
    range: Range,
) -> Result<impl Iterator<Item = Result<impl Borrow<T::Value> + 'tx, RuntimeError>>, RuntimeError>
where
    Range: RangeBounds<T::Key> + RangeBounds<&'tx T::Key> + 'tx,
{
    /// TODO
    struct Iter<'tx, T: Table> {
        /// TODO
        iter: heed::RoRange<'tx, StorableHeed<T::Key>, StorableHeed<T::Value>>,
    }

    // TODO
    impl<'tx, T: Table> Iterator for Iter<'tx, T> {
        type Item = Result<&'tx T::Value, RuntimeError>;
        fn next(&mut self) -> Option<Self::Item> {
            // TODO
            self.iter
                .next()
                .map(|result| result.map(|value| value.1).map_err(RuntimeError::from))
        }
    }

    // TODO
    match db.range(tx_ro, &range) {
        Ok(iter) => Ok(Iter::<'_, T> { iter }),
        Err(e) => Err(RuntimeError::from(e)),
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRo Impl
impl<'tx, T: Table> DatabaseRo<'tx, T> for HeedTableRo<'tx, T> {
    #[inline]
    fn get(&'tx self, key: &'_ T::Key) -> Result<impl Borrow<T::Value> + 'tx, RuntimeError> {
        get::<T>(&self.db, self.tx_ro, key)
    }

    #[allow(clippy::trait_duplication_in_bounds)]
    fn get_range<Range>(
        &'tx self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<impl Borrow<T::Value> + 'tx, RuntimeError>>, RuntimeError>
    where
        Range: RangeBounds<T::Key> + RangeBounds<&'tx T::Key> + 'tx,
    {
        get_range::<T, Range>(&self.db, self.tx_ro, range)
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRw Impl
impl<'tx, T: Table> DatabaseRo<'tx, T> for HeedTableRw<'tx, T> {
    fn get(&'tx self, key: &'_ T::Key) -> Result<impl Borrow<T::Value> + 'tx, RuntimeError> {
        get::<T>(&self.db, self.tx_rw, key)
    }

    #[allow(clippy::trait_duplication_in_bounds)]
    fn get_range<Range>(
        &'tx self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<impl Borrow<T::Value> + 'tx, RuntimeError>>, RuntimeError>
    where
        Range: RangeBounds<T::Key> + RangeBounds<&'tx T::Key> + 'tx,
    {
        get_range::<T, Range>(&self.db, self.tx_rw, range)
    }
}

impl<'tx, T: Table> DatabaseRw<'tx, T> for HeedTableRw<'tx, T> {
    fn put(&mut self, key: &T::Key, value: &T::Value) -> Result<(), RuntimeError> {
        Ok(self.db.put(self.tx_rw, key, value)?)
    }

    fn delete(&mut self, key: &T::Key) -> Result<(), RuntimeError> {
        self.db.delete(self.tx_rw, key)?;
        Ok(())
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

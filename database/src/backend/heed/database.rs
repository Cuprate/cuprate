//! Implementation of `trait Database` for `heed`.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::{Borrow, Cow},
    fmt::Debug,
    ops::RangeBounds,
    sync::RwLockReadGuard,
};

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
pub(super) struct HeedTableRo<'tx, T: Table>
where
    <T as Table>::Key: ToOwned + Debug,
    <<T as Table>::Key as ToOwned>::Owned: Debug,
    <T as Table>::Value: ToOwned + Debug,
    <<T as Table>::Value as ToOwned>::Owned: Debug,
    <<T as Table>::Key as crate::Key>::Primary: ToOwned + Debug,
    <<<T as Table>::Key as crate::Key>::Primary as ToOwned>::Owned: Debug,
{
    /// An already opened database table.
    pub(super) db: HeedDb<T::Key, T::Value>,
    /// The associated read-only transaction that opened this table.
    pub(super) tx_ro: &'tx heed::RoTxn<'tx>,
}

/// An opened read/write database associated with a transaction.
///
/// Matches `redb::Table` (read & write).
pub(super) struct HeedTableRw<'env, 'tx, T: Table>
where
    <T as Table>::Key: ToOwned + Debug,
    <<T as Table>::Key as ToOwned>::Owned: Debug,
    <T as Table>::Value: ToOwned + Debug,
    <<T as Table>::Value as ToOwned>::Owned: Debug,
    <<T as Table>::Key as crate::Key>::Primary: ToOwned + Debug,
    <<<T as Table>::Key as crate::Key>::Primary as ToOwned>::Owned: Debug,
{
    /// TODO
    pub(super) db: HeedDb<T::Key, T::Value>,
    /// The associated read/write transaction that opened this table.
    pub(super) tx_rw: &'tx mut heed::RwTxn<'env>,
}

//---------------------------------------------------------------------------------------------------- Shared functions
// FIXME: we cannot just deref `HeedTableRw -> HeedTableRo` and
// call the functions since the database is held by value, so
// just use these generic functions that both can call instead.

/// Shared generic `get()` between `HeedTableR{o,w}`.
#[inline]
fn get<'a, 'tx, T: Table>(
    db: &'_ HeedDb<T::Key, T::Value>,
    tx_ro: &'tx heed::RoTxn<'_>,
    key: &T::Key,
    access_guard: &'a mut Option<Cow<'tx, T::Value>>,
) -> Result<Cow<'a, T::Value>, RuntimeError>
where
    <T as Table>::Key: ToOwned + Debug,
    <<T as Table>::Key as ToOwned>::Owned: Debug,
    <T as Table>::Value: ToOwned + Debug,
    <<T as Table>::Value as ToOwned>::Owned: Debug,
    <<T as Table>::Key as crate::Key>::Primary: ToOwned + Debug,
    <<<T as Table>::Key as crate::Key>::Primary as ToOwned>::Owned: Debug,
{
    match db.get(tx_ro, key) {
        Ok(Some(value)) => {
            *access_guard = Some(value);
            Ok(Cow::Borrowed(access_guard.as_ref().unwrap()))
        }
        Ok(None) => Err(RuntimeError::KeyNotFound),
        Err(e) => Err(e.into()),
    }
}

/// Shared generic `get_range()` between `HeedTableR{o,w}`.
#[inline]
#[allow(clippy::needless_pass_by_value, clippy::trait_duplication_in_bounds)]
fn get_range<'a, T: Table, Range>(
    db: &'a HeedDb<T::Key, T::Value>,
    tx_ro: &'a heed::RoTxn<'_>,
    range: Range,
) -> Result<impl Iterator<Item = Result<Cow<'a, T::Value>, RuntimeError>>, RuntimeError>
where
    Range: RangeBounds<T::Key> + RangeBounds<Cow<'a, T::Key>> + 'a,
    <T as Table>::Key: ToOwned + Debug,
    <<T as Table>::Key as ToOwned>::Owned: Debug,
    <T as Table>::Value: ToOwned + Debug,
    <<T as Table>::Value as ToOwned>::Owned: Debug,
    <<T as Table>::Key as crate::Key>::Primary: ToOwned + Debug,
    <<<T as Table>::Key as crate::Key>::Primary as ToOwned>::Owned: Debug,
{
    Ok(db.range(tx_ro, &range)?.map(|res| Ok(res?.1)))
}

//---------------------------------------------------------------------------------------------------- DatabaseRo Impl
impl<'tx, T: Table> DatabaseRo<'tx, T> for HeedTableRo<'tx, T>
where
    <T as Table>::Key: ToOwned + Debug,
    <<T as Table>::Key as ToOwned>::Owned: Debug,
    <T as Table>::Value: ToOwned + Debug,
    <<T as Table>::Value as ToOwned>::Owned: Debug,
    <<T as Table>::Key as crate::Key>::Primary: ToOwned + Debug,
    <<<T as Table>::Key as crate::Key>::Primary as ToOwned>::Owned: Debug,
{
    type ValueGuard<'a> = Cow<'a, T::Value> where Self: 'a;

    #[inline]
    fn get<'a, 'b>(
        &'a self,
        key: &'a T::Key,
        access_guard: &'b mut Option<Self::ValueGuard<'a>>,
    ) -> Result<Cow<'b, T::Value>, RuntimeError> {
        get::<T>(&self.db, self.tx_ro, key, access_guard)
    }

    #[inline]
    #[allow(clippy::trait_duplication_in_bounds)]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<Cow<'a, T::Value>, RuntimeError>>, RuntimeError>
    where
        Range: RangeBounds<T::Key> + RangeBounds<Cow<'a, T::Key>> + 'a,
    {
        get_range::<T, Range>(&self.db, self.tx_ro, range)
    }
}

//---------------------------------------------------------------------------------------------------- DatabaseRw Impl
impl<'tx, T: Table> DatabaseRo<'tx, T> for HeedTableRw<'_, 'tx, T>
where
    <T as Table>::Key: ToOwned + Debug,
    <<T as Table>::Key as ToOwned>::Owned: Debug,
    <T as Table>::Value: ToOwned + Debug,
    <<T as Table>::Value as ToOwned>::Owned: Debug,
    <<T as Table>::Key as crate::Key>::Primary: ToOwned + Debug,
    <<<T as Table>::Key as crate::Key>::Primary as ToOwned>::Owned: Debug,
{
    type ValueGuard<'a> = Cow<'a, T::Value> where Self: 'a;

    #[inline]
    fn get<'a, 'b>(
        &'a self,
        key: &'a T::Key,
        access_guard: &'b mut Option<Self::ValueGuard<'a>>,
    ) -> Result<Cow<'b, T::Value>, RuntimeError> {
        get::<T>(&self.db, self.tx_rw, key, access_guard)
    }

    #[inline]
    #[allow(clippy::trait_duplication_in_bounds)]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<Cow<'a, T::Value>, RuntimeError>>, RuntimeError>
    where
        Range: RangeBounds<T::Key> + RangeBounds<Cow<'a, T::Key>> + 'a,
    {
        get_range::<T, Range>(&self.db, self.tx_rw, range)
    }
}

impl<'env, 'tx, T: Table> DatabaseRw<'env, 'tx, T> for HeedTableRw<'env, 'tx, T>
where
    <T as Table>::Key: ToOwned + Debug,
    <<T as Table>::Key as ToOwned>::Owned: Debug,
    <T as Table>::Value: ToOwned + Debug,
    <<T as Table>::Value as ToOwned>::Owned: Debug,
    <<T as Table>::Key as crate::Key>::Primary: ToOwned + Debug,
    <<<T as Table>::Key as crate::Key>::Primary as ToOwned>::Owned: Debug,
{
    #[inline]
    fn put(&mut self, key: &T::Key, value: &T::Value) -> Result<(), RuntimeError> {
        Ok(self.db.put(self.tx_rw, key, value)?)
    }

    #[inline]
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

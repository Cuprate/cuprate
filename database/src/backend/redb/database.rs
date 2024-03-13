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
    value_guard::ValueGuard,
    ToOwnedDebug,
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
    db.get(Cow::Borrowed(key))?.ok_or(RuntimeError::KeyNotFound)
}

/// Shared generic `get_range()` between `RedbTableR{o,w}`.
#[inline]
fn get_range<'a, T: Table, Range>(
    db: &'a impl redb::ReadableTable<StorableRedb<T::Key>, StorableRedb<T::Value>>,
    range: &'a Range,
) -> Result<
    impl Iterator<Item = Result<redb::AccessGuard<'a, StorableRedb<T::Value>>, RuntimeError>> + 'a,
    RuntimeError,
>
where
    Range: RangeBounds<T::Key> + 'a,
{
    /// HACK: `redb` sees the database's key type as `Cow<'_, T::Key>`,
    /// not `T::Key` directly like `heed` does. As such, it wants the
    /// range to be over `Cow<'_, T::Key>`, not `T::Key` directly.
    ///
    /// If `DatabaseRo` were to want `Cow<'_, T::Key>` as input in `get()`,
    /// `get_range()`, it would complicate the API:
    /// ```rust,ignore
    /// // This would be needed...
    /// let range = Cow::Owned(0)..Cow::Owned(1);
    /// // ...instead of the more obvious
    /// let range = 0..1;
    /// ```
    ///
    /// As such, `DatabaseRo` only wants `RangeBounds<T::Key>` and
    /// we create a compatibility struct here, essentially converting
    /// this functions input:
    /// ```rust,ignore
    /// RangeBound<T::Key>
    /// ```
    /// into `redb`'s desired:
    /// ```rust,ignore
    /// RangeBound<Cow<'_, T::Key>>
    /// ```
    struct CowRange<'a, K>
    where
        K: ToOwnedDebug,
    {
        /// The start bound of `Range`.
        start_bound: Bound<Cow<'a, K>>,
        /// The end bound of `Range`.
        end_bound: Bound<Cow<'a, K>>,
    }

    /// This impl forwards our `T::Key` to be wrapped in a Cow.
    impl<'a, K> RangeBounds<Cow<'a, K>> for CowRange<'a, K>
    where
        K: ToOwnedDebug,
    {
        fn start_bound(&self) -> Bound<&Cow<'a, K>> {
            self.start_bound.as_ref()
        }

        fn end_bound(&self) -> Bound<&Cow<'a, K>> {
            self.end_bound.as_ref()
        }
    }

    let start_bound = match range.start_bound() {
        Bound::Included(t) => Bound::Included(Cow::Borrowed(t)),
        Bound::Excluded(t) => Bound::Excluded(Cow::Borrowed(t)),
        Bound::Unbounded => Bound::Unbounded,
    };
    let end_bound = match range.end_bound() {
        Bound::Included(t) => Bound::Included(Cow::Borrowed(t)),
        Bound::Excluded(t) => Bound::Excluded(Cow::Borrowed(t)),
        Bound::Unbounded => Bound::Unbounded,
    };
    let range = CowRange {
        start_bound,
        end_bound,
    };

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
    fn get_range<'a, Range>(
        &'a self,
        range: &'a Range,
    ) -> Result<
        impl Iterator<Item = Result<impl ValueGuard<T::Value>, RuntimeError>> + 'a,
        RuntimeError,
    >
    where
        Range: RangeBounds<T::Key> + 'a,
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
    fn get_range<'a, Range>(
        &'a self,
        range: &'a Range,
    ) -> Result<
        impl Iterator<Item = Result<impl ValueGuard<T::Value>, RuntimeError>> + 'a,
        RuntimeError,
    >
    where
        Range: RangeBounds<T::Key> + 'a,
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

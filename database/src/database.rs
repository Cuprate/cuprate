//! Abstracted database table operations; `trait DatabaseRo` & `trait DatabaseRw`.

//---------------------------------------------------------------------------------------------------- Import
use std::ops::RangeBounds;

use crate::{error::RuntimeError, table::Table};

//---------------------------------------------------------------------------------------------------- DatabaseIter
/// Generic post-fix documentation for `DatabaseIter` methods.
macro_rules! doc_iter {
    () => {
        r"Although the returned iterator itself is tied to the lifetime
of `&self`, the returned values from the iterator are _owned_.

# Errors
Each key in the `range` has the potential to error, for example,
if a particular key in the `range` does not exist,
[`RuntimeError::KeyNotFound`] wrapped in [`Err`] will be returned
from the iterator."
    };
}

/// Database (key-value store) read-only iteration abstraction.
///
/// These are read-only iteration-related operations that
/// can only be called from [`DatabaseRo`] objects.
///
/// # Hack
/// This is a HACK to get around the fact our read/write tables
/// cannot safely return values returning lifetimes, as such,
/// only read-only tables implement this trait.
///
/// - <https://github.com/Cuprate/cuprate/pull/102#discussion_r1548695610>
/// - <https://github.com/Cuprate/cuprate/pull/104>
pub trait DatabaseIter<T: Table> {
    /// Get an [`Iterator`] of value's corresponding to a range of keys.
    ///
    /// For example:
    /// ```rust,ignore
    /// // This will return all 100 values corresponding
    /// // to the keys `{0, 1, 2, ..., 100}`.
    /// self.get_range(0..100);
    /// ```
    ///
    /// Although the returned iterator itself is tied to the lifetime
    /// of `&'a self`, the returned values from the iterator are _owned_.
    ///
    #[doc = doc_iter!()]
    #[allow(clippy::iter_not_returning_iterator)]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<T::Value, RuntimeError>> + 'a, RuntimeError>
    where
        Range: RangeBounds<T::Key> + 'a;

    /// Get an [`Iterator`] that returns the `(key, value)` types for this database.
    #[doc = doc_iter!()]
    #[allow(clippy::iter_not_returning_iterator)]
    fn iter(
        &self,
    ) -> Result<impl Iterator<Item = Result<(T::Key, T::Value), RuntimeError>> + '_, RuntimeError>;

    /// Get an [`Iterator`] that returns _only_ the `key` type for this database.
    #[doc = doc_iter!()]
    fn keys(&self)
        -> Result<impl Iterator<Item = Result<T::Key, RuntimeError>> + '_, RuntimeError>;

    /// Get an [`Iterator`] that returns _only_ the `value` type for this database.
    #[doc = doc_iter!()]
    fn values(
        &self,
    ) -> Result<impl Iterator<Item = Result<T::Value, RuntimeError>> + '_, RuntimeError>;
}

//---------------------------------------------------------------------------------------------------- DatabaseRo
/// Generic post-fix documentation for `DatabaseR{o,w}` methods.
macro_rules! doc_database {
    () => {
        r"# Errors
This will return [`RuntimeError::KeyNotFound`] wrapped in [`Err`] if `key` does not exist.

It will return other [`RuntimeError`]'s on things like IO errors as well."
    };
}

/// Database (key-value store) read abstraction.
///
/// This is a read-only database table,
/// write operations are defined in [`DatabaseRw`].
///
/// # Safety
/// The table type that implements this MUST be `Send`.
///
/// However if the table holds a reference to a transaction:
/// - only the transaction only has to be `Send`
/// - the table cannot implement `Send`
///
/// For example:
///
/// `heed`'s transactions are `Send` but `HeedTableRo` contains a `&`
/// to the transaction, as such, if `Send` were implemented on `HeedTableRo`
/// then 1 transaction could be used to open multiple tables, then sent to
/// other threads - this would be a soundness hole against `HeedTableRo`.
///
/// `&T` is only `Send` if `T: Sync`.
///
/// `heed::RoTxn: !Sync`, therefore our table
/// holding `&heed::RoTxn` must NOT be `Send`.
///
/// - <https://doc.rust-lang.org/std/marker/trait.Sync.html>
/// - <https://doc.rust-lang.org/nomicon/send-and-sync.html>
pub unsafe trait DatabaseRo<T: Table> {
    /// Get the value corresponding to a key.
    #[doc = doc_database!()]
    fn get(&self, key: &T::Key) -> Result<T::Value, RuntimeError>;

    /// Returns `true` if the database contains a value for the specified key.
    ///
    /// # Errors
    /// Note that this will _never_ return `Err(RuntimeError::KeyNotFound)`,
    /// as in that case, `Ok(false)` will be returned.
    ///
    /// Other errors may still occur.
    fn contains(&self, key: &T::Key) -> Result<bool, RuntimeError> {
        match self.get(key) {
            Ok(_) => Ok(true),
            Err(RuntimeError::KeyNotFound) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Returns the number of `(key, value)` pairs in the database.
    #[doc = doc_database!()]
    fn len(&self) -> Result<u64, RuntimeError>;

    /// Returns the first `(key, value)` pair in the database.
    #[doc = doc_database!()]
    fn first(&self) -> Result<(T::Key, T::Value), RuntimeError>;

    /// Returns the last `(key, value)` pair in the database.
    #[doc = doc_database!()]
    fn last(&self) -> Result<(T::Key, T::Value), RuntimeError>;

    /// Returns `true` if the database contains no `(key, value)` pairs.
    #[doc = doc_database!()]
    fn is_empty(&self) -> Result<bool, RuntimeError>;
}

//---------------------------------------------------------------------------------------------------- DatabaseRw
/// Database (key-value store) read/write abstraction.
///
/// All [`DatabaseRo`] functions are also callable by [`DatabaseRw`].
pub trait DatabaseRw<T: Table>: DatabaseRo<T> {
    /// Insert a key-value pair into the database.
    ///
    /// This will overwrite any existing key-value pairs.
    ///
    #[doc = doc_database!()]
    ///
    /// Note that this will never [`RuntimeError::KeyExists`].
    fn put(&mut self, key: &T::Key, value: &T::Value) -> Result<(), RuntimeError>;

    /// Delete a key-value pair in the database.
    ///
    /// This will return `Ok(())` if the key does not exist.
    ///
    #[doc = doc_database!()]
    ///
    /// Note that this will never [`RuntimeError::KeyExists`].
    fn delete(&mut self, key: &T::Key) -> Result<(), RuntimeError>;

    /// Delete and return a key-value pair in the database.
    ///
    /// This is the same as [`DatabaseRw::delete`], however,
    /// it will serialize the `T::Value` and return it.
    ///
    #[doc = doc_database!()]
    fn take(&mut self, key: &T::Key) -> Result<T::Value, RuntimeError>;

    /// Fetch the value, and apply a function to it - or delete the entry.
    ///
    /// This will call [`DatabaseRo::get`] and call your provided function `f` on it.
    ///
    /// The [`Option`] `f` returns will dictate whether `update()`:
    /// - Updates the current value OR
    /// - Deletes the `(key, value)` pair
    ///
    /// - If `f` returns `Some(value)`, that will be [`DatabaseRw::put`] as the new value
    /// - If `f` returns `None`, the entry will be [`DatabaseRw::delete`]d
    ///
    #[doc = doc_database!()]
    fn update<F>(&mut self, key: &T::Key, mut f: F) -> Result<(), RuntimeError>
    where
        F: FnMut(T::Value) -> Option<T::Value>,
    {
        let value = DatabaseRo::get(self, key)?;

        match f(value) {
            Some(value) => DatabaseRw::put(self, key, &value),
            None => DatabaseRw::delete(self, key),
        }
    }

    /// Removes and returns the first `(key, value)` pair in the database.
    ///
    #[doc = doc_database!()]
    fn pop_first(&mut self) -> Result<(T::Key, T::Value), RuntimeError>;

    /// Removes and returns the last `(key, value)` pair in the database.
    ///
    #[doc = doc_database!()]
    fn pop_last(&mut self) -> Result<(T::Key, T::Value), RuntimeError>;
}

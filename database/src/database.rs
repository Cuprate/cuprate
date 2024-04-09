//! Abstracted database; `trait DatabaseRo` & `trait DatabaseRw`.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::{Borrow, Cow},
    fmt::Debug,
    ops::{Deref, RangeBounds},
};

use crate::{
    error::RuntimeError,
    table::Table,
    transaction::{TxRo, TxRw},
};

//---------------------------------------------------------------------------------------------------- DatabaseIter
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
    /// Get an iterator of value's corresponding to a range of keys.
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
    /// # Errors
    /// Each key in the `range` has the potential to error, for example,
    /// if a particular key in the `range` does not exist,
    /// [`RuntimeError::KeyNotFound`] wrapped in [`Err`] will be returned
    /// from the iterator.
    #[allow(clippy::iter_not_returning_iterator)]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<T::Value, RuntimeError>> + 'a, RuntimeError>
    where
        Range: RangeBounds<T::Key> + 'a;

    /// TODO
    ///
    /// # Errors
    /// TODO
    #[allow(clippy::iter_not_returning_iterator)]
    fn iter(
        &self,
    ) -> Result<impl Iterator<Item = Result<(T::Key, T::Value), RuntimeError>> + '_, RuntimeError>;

    /// TODO
    ///
    /// # Errors
    /// TODO
    fn keys(&self)
        -> Result<impl Iterator<Item = Result<T::Key, RuntimeError>> + '_, RuntimeError>;

    /// TODO
    ///
    /// # Errors
    /// TODO
    fn values(
        &self,
    ) -> Result<impl Iterator<Item = Result<T::Value, RuntimeError>> + '_, RuntimeError>;
}

//---------------------------------------------------------------------------------------------------- DatabaseRo
/// Database (key-value store) read abstraction.
///
/// This is a read-only database table,
/// write operations are defined in [`DatabaseRw`].
pub trait DatabaseRo<T: Table> {
    /// Get the value corresponding to a key.
    ///
    /// The returned value is _owned_.
    ///
    /// # Errors
    /// This will return [`RuntimeError::KeyNotFound`] wrapped in [`Err`] if `key` does not exist.
    ///
    /// It will return other [`RuntimeError`]'s on things like IO errors as well.
    fn get(&self, key: &T::Key) -> Result<T::Value, RuntimeError>;

    /// TODO
    ///
    /// # Errors
    /// TODO
    fn contains(&self, key: &T::Key) -> Result<bool, RuntimeError> {
        match self.get(key) {
            Ok(_) => Ok(true),
            Err(RuntimeError::KeyNotFound) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// TODO
    ///
    /// # Errors
    /// TODO
    fn len(&self) -> Result<u64, RuntimeError>;

    /// TODO
    ///
    /// # Errors
    /// TODO
    fn first(&self) -> Result<(T::Key, T::Value), RuntimeError>;

    /// TODO
    ///
    /// # Errors
    /// TODO
    fn last(&self) -> Result<(T::Key, T::Value), RuntimeError>;

    /// TODO
    ///
    /// # Errors
    /// TODO
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
    /// # Errors
    /// This will not return [`RuntimeError::KeyExists`].
    fn put(&mut self, key: &T::Key, value: &T::Value) -> Result<(), RuntimeError>;

    /// Delete a key-value pair in the database.
    ///
    /// # Errors
    /// This will return [`RuntimeError::KeyNotFound`] wrapped in [`Err`] if `key` does not exist.
    fn delete(&mut self, key: &T::Key) -> Result<(), RuntimeError>;

    /// Delete and return a key-value pair in the database.
    ///
    /// This is the same as [`DatabaseRw::delete`], however,
    /// it will serialize the `T::Value` and return it.
    ///
    /// # Errors
    /// This will return [`RuntimeError::KeyNotFound`] wrapped in [`Err`] if `key` does not exist.
    fn take(&mut self, key: &T::Key) -> Result<T::Value, RuntimeError>;

    /// TODO
    ///
    /// # Errors
    /// TODO
    fn pop_first(&mut self) -> Result<(T::Key, T::Value), RuntimeError>;

    /// TODO
    ///
    /// # Errors
    /// TODO
    fn pop_last(&mut self) -> Result<(T::Key, T::Value), RuntimeError>;
}

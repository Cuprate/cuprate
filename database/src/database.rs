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

    /// Get an iterator of `(key, value)`s corresponding to a range of keys.
    ///
    /// For example:
    /// ```rust,ignore
    /// // This will return all 100 tuples of `(key, value)` where
    /// // `key` is `0..100` and `value` is the corresponding value.
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
    ) -> Result<impl Iterator<Item = Result<(T::Key, T::Value), RuntimeError>> + 'a, RuntimeError>
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

    /// TODO
    ///
    /// # Errors
    /// TODO
    fn clear(&mut self) -> Result<(), RuntimeError>;

    /// TODO
    ///
    /// - `true == keep`
    /// - `false == remove`
    ///
    /// # Errors
    /// TODO
    fn retain<P>(&mut self, predicate: P) -> Result<(), RuntimeError>
    where
        P: FnMut(T::Key, T::Value) -> bool;
}

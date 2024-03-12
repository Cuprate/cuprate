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
    value_guard::ValueGuard,
};

//---------------------------------------------------------------------------------------------------- DatabaseRo
/// Database (key-value store) read abstraction.
///
/// This is a read-only database table,
/// write operations are defined in [`DatabaseRw`].
pub trait DatabaseRo<'tx, T: Table> {
    /// Get the value corresponding to a key.
    ///
    /// This returns a guard to the value, not the value itself.
    /// See [`ValueGuard`] for more info.
    ///
    /// # Errors
    /// This will return [`RuntimeError::KeyNotFound`] wrapped in [`Err`] if `key` does not exist.
    ///
    /// It will return other [`RuntimeError`]'s on things like IO errors as well.
    fn get<'a>(&'a self, key: &'a T::Key) -> Result<impl ValueGuard<T::Value> + 'a, RuntimeError>;

    /// Get an iterator of values corresponding to a range of keys.
    ///
    /// This returns guards to the values, not the values themselves.
    /// See [`ValueGuard`] for more info.
    ///
    /// # Errors
    /// Each key in the `range` has the potential to error, for example,
    /// if a particular key in the `range` does not exist,
    /// [`RuntimeError::KeyNotFound`] wrapped in [`Err`] will be returned
    /// from the iterator.
    #[allow(clippy::trait_duplication_in_bounds)]
    fn get_range<'a, Range>(
        &'a self,
        range: &'a Range,
    ) -> Result<
        impl Iterator<Item = Result<impl ValueGuard<T::Value>, RuntimeError>> + 'a,
        RuntimeError,
    >
    where
        Range: RangeBounds<T::Key> + 'a;
}

//---------------------------------------------------------------------------------------------------- DatabaseRw
/// Database (key-value store) read/write abstraction.
///
/// All [`DatabaseRo`] functions are also callable by [`DatabaseRw`].
pub trait DatabaseRw<'env, 'tx, T: Table>: DatabaseRo<'tx, T> {
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
}

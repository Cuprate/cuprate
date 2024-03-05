//! Abstracted database; `trait DatabaseRo` & `trait DatabaseRw`.

//---------------------------------------------------------------------------------------------------- Import
use std::ops::Deref;

use crate::{error::RuntimeError, table::Table, value_guard::ValueGuard};

//---------------------------------------------------------------------------------------------------- DatabaseRo
/// Database (key-value store) read abstraction.
///
/// TODO: document relation between `DatabaseRo` <-> `DatabaseRw`.
pub trait DatabaseRo<'tx, T: Table> {
    /// TODO
    /// # Errors
    /// TODO
    ///
    /// This will return [`RuntimeError::KeyNotFound`] wrapped in [`Err`] if `key` does not exist.
    fn get(&'tx self, key: &'_ T::Key) -> Result<impl ValueGuard<'tx, T::Value>, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn get_range<R: std::ops::RangeBounds<T::Key>>(
        &self,
        range: R,
    ) -> impl Iterator<Item = Result<impl ValueGuard<'tx, T::Value>, RuntimeError>>;
}

//---------------------------------------------------------------------------------------------------- DatabaseRw
/// Database (key-value store) read/write abstraction.
///
/// TODO: document relation between `DatabaseRo` <-> `DatabaseRw`.
pub trait DatabaseRw<'tx, T: Table>: DatabaseRo<'tx, T> {
    /// TODO
    /// # Errors
    /// TODO
    fn put(&mut self, key: &T::Key, value: &T::Value) -> Result<(), RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    ///
    /// This will return [`RuntimeError::KeyNotFound`] wrapped in [`Err`] if `key` does not exist.
    fn delete(&mut self, key: &T::Key) -> Result<(), RuntimeError>;
}

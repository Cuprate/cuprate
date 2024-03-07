//! Abstracted database; `trait DatabaseRo` & `trait DatabaseRw`.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::Borrow,
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
/// TODO: document relation between `DatabaseRo` <-> `DatabaseRw`.
pub trait DatabaseRo<'env, 'tx, T: Table> {
    /// TODO
    /// # Errors
    /// TODO
    ///
    /// This will return [`RuntimeError::KeyNotFound`] wrapped in [`Err`] if `key` does not exist.
    fn get<'a>(&'a self, key: &'a T::Key) -> Result<impl Borrow<T::Value> + 'a, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    #[allow(clippy::trait_duplication_in_bounds)]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<impl Borrow<T::Value> + 'a, RuntimeError>>, RuntimeError>
    where
        // FIXME:
        // - `RangeBounds<T::Key>` is to satisfy `heed` bounds
        // - `RangeBounds<&'a T::Key> + 'a` is to satisfy `redb` bounds
        Range: RangeBounds<T::Key> + RangeBounds<&'a T::Key> + 'a;
}

//---------------------------------------------------------------------------------------------------- DatabaseRw
/// Database (key-value store) read/write abstraction.
///
/// TODO: document relation between `DatabaseRo` <-> `DatabaseRw`.
pub trait DatabaseRw<'env, 'tx, T: Table>: DatabaseRo<'env, 'tx, T> {
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

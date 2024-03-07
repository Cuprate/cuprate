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
pub trait DatabaseRo<'tx, T: Table> {
    /// TODO
    /// # Errors
    /// TODO
    ///
    /// This will return [`RuntimeError::KeyNotFound`] wrapped in [`Err`] if `key` does not exist.
    fn get(&'tx self, key: &'_ T::Key) -> Result<impl Borrow<T::Value> + 'tx, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    #[allow(clippy::trait_duplication_in_bounds)]
    fn get_range<Range>(
        &'tx self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<impl Borrow<T::Value> + 'tx, RuntimeError>>, RuntimeError>
    where
        // FIXME:
        // - `RangeBounds<T::Key>` is to satisfy `heed` bounds
        // - `RangeBounds<&T::Key> + 'tx` is to satisfy `redb` bounds
        Range: RangeBounds<T::Key> + RangeBounds<&'tx T::Key> + 'tx;
}

//---------------------------------------------------------------------------------------------------- DatabaseRw
/// Database (key-value store) read/write abstraction.
///
/// TODO: document relation between `DatabaseRo` <-> `DatabaseRw`.
pub trait DatabaseRw<'db, 'tx, T: Table>: DatabaseRo<'tx, T> {
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

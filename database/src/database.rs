//! Abstracted database; `trait DatabaseRo` & `trait DatabaseRw`.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::Borrow,
    ops::{Deref, RangeBounds},
};

use crate::{error::RuntimeError, table::Table};

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
    fn get_range<Key, Range>(
        &'tx self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<impl Borrow<T::Value> + 'tx, RuntimeError>>, RuntimeError>
    where
        // FIXME:
        // - `Key` + `RangeBounds<Key>` is to satisfy `redb` bounds
        // - `RangeBounds<T::Key>` is to satisfy `heed` bounds
        //
        // Abstracting over different bounds leads to type soup :)
        Key: Borrow<&'tx T::Key> + 'tx,
        Range: RangeBounds<T::Key> + RangeBounds<Key> + 'tx;
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

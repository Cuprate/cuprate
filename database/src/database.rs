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
/// TODO: document relation between `DatabaseRo` <-> `DatabaseRw`.
///
/// TODO: document these trait bounds...
pub trait DatabaseRo<'tx, T: Table>
where
    <T as Table>::Key: ToOwned + Debug,
    <<T as Table>::Key as ToOwned>::Owned: Debug,
    <T as Table>::Value: ToOwned + Debug,
    <<T as Table>::Value as ToOwned>::Owned: Debug,
    <<T as Table>::Key as crate::Key>::Primary: ToOwned + Debug,
    <<<T as Table>::Key as crate::Key>::Primary as ToOwned>::Owned: Debug,
{
    /// A guard for accessing database values.
    ///
    /// TODO: explain this stupid thing
    type ValueGuard<'a>
    where
        Self: 'a;

    /// TODO
    /// # Errors
    /// TODO
    ///
    /// This will return [`RuntimeError::KeyNotFound`] wrapped in [`Err`] if `key` does not exist.
    fn get<'a, 'b>(
        &'a self,
        key: &'a T::Key,
        access_guard: &'b mut Option<Self::ValueGuard<'a>>,
    ) -> Result<Cow<'b, T::Value>, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    #[allow(clippy::trait_duplication_in_bounds)]
    fn get_range<'a, Range>(
        &'a self,
        range: Range,
    ) -> Result<impl Iterator<Item = Result<Cow<'a, T::Value>, RuntimeError>>, RuntimeError>
    where
        // FIXME:
        // - `RangeBounds<T::Key>` is to satisfy `heed` bounds
        // - `RangeBounds<&'a T::Key> + 'a` is to satisfy `redb` bounds
        Range: RangeBounds<T::Key> + RangeBounds<Cow<'a, T::Key>> + 'a;
}

//---------------------------------------------------------------------------------------------------- DatabaseRw
/// Database (key-value store) read/write abstraction.
///
/// TODO: document relation between `DatabaseRo` <-> `DatabaseRw`.
///
/// TODO: document these trait bounds...
pub trait DatabaseRw<'env, 'tx, T: Table>: DatabaseRo<'tx, T>
where
    <T as Table>::Key: ToOwned + Debug,
    <<T as Table>::Key as ToOwned>::Owned: Debug,
    <T as Table>::Value: ToOwned + Debug,
    <<T as Table>::Value as ToOwned>::Owned: Debug,
    <<T as Table>::Key as crate::Key>::Primary: ToOwned + Debug,
    <<<T as Table>::Key as crate::Key>::Primary as ToOwned>::Owned: Debug,
{
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

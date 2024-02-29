//! Abstracted database; `trait DatabaseRo` & `trait DatabaseRw`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{error::RuntimeError, table::Table};

//---------------------------------------------------------------------------------------------------- DatabaseRo
/// Database (key-value store) read abstraction.
///
/// TODO: document relation between `DatabaseRo` <-> `DatabaseRw`.
pub trait DatabaseRo<T: Table> {
    /// TODO
    /// # Errors
    /// TODO
    fn get(&self, key: &T::Key) -> Result<Option<&T::Value>, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    //
    // TODO: (Iterators + ?Sized + lifetimes) == bad time
    // fix this later.
    fn get_range<'a>(
        &'a self,
        key: &'a T::Key,
        amount: usize,
    ) -> Result<impl Iterator<Item = &'a T::Value>, RuntimeError>
    where
        <T as Table>::Value: 'a;
}

//---------------------------------------------------------------------------------------------------- DatabaseRw
/// Database (key-value store) read/write abstraction.
///
/// TODO: document relation between `DatabaseRo` <-> `DatabaseRw`.
pub trait DatabaseRw<T: Table>: DatabaseRo<T> {
    /// TODO
    /// # Errors
    /// TODO
    fn put(&mut self, key: &T::Key, value: &T::Value) -> Result<(), RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn clear(&mut self) -> Result<(), RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn delete(&mut self, key: &T::Key) -> Result<bool, RuntimeError>;
}

//! Abstracted database; `trait DatabaseRead` & `trait DatabaseWrite`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{error::RuntimeError, table::Table};

//---------------------------------------------------------------------------------------------------- DatabaseRead
/// Database (key-value store) read abstraction.
///
/// TODO
pub trait DatabaseRead<T: Table> {
    /// TODO
    /// # Errors
    /// TODO
    fn get(&self, key: &T::Key) -> Result<Option<T::Value>, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn get_range(
        &self,
        key: &T::Key,
        amount: usize,
    ) -> Result<impl Iterator<Item = T::Value>, RuntimeError>;
}

//---------------------------------------------------------------------------------------------------- DatabaseWrite
/// Database (key-value store) write abstraction.
///
/// TODO
pub trait DatabaseWrite<T: Table>: DatabaseRead<T> {
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

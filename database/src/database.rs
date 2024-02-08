//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::{error::RuntimeError, table::Table};

// use std::{marker::PhantomData, path::Path};

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- Database
/// Database (key-value store) abstraction.
///
/// TODO
pub trait Database<T: Table> {
    //-------------------------------------------------------- Read
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

    //-------------------------------------------------------- Write
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

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

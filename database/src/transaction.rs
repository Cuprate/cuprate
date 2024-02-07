//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::error::RuntimeError;

// use std::{marker::PhantomData, path::Path};

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- RoTx
/// TODO
///
/// Read-only transaction.
pub trait RoTx<'db, K, V> {
    /// TODO
    /// # Errors
    /// TODO
    fn get(&self, key: &K) -> Result<Option<V>, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn get_range(&self, key: &K, amount: usize) -> Result<impl Iterator<Item = V>, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn commit(self) -> Result<(), RuntimeError>;
}

//---------------------------------------------------------------------------------------------------- RwTx
/// TODO
///
/// Read/Write transaction.
pub trait RwTx<'db, K, V> {
    /// TODO
    /// # Errors
    /// TODO
    fn put(&mut self, key: &K, value: &V) -> Result<(), RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn clear(&mut self) -> Result<(), RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn delete(&mut self, key: &K) -> Result<bool, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn commit(self) -> Result<(), RuntimeError>;

    /// TODO
    fn abort(self);
}

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

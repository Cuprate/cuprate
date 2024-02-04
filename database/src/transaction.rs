//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::error::RuntimeError;

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- Read-only Transaction
/// TODO
///
/// Read-only transaction.
pub trait RoTx<'env, K, V> {
    /// TODO
    /// # Errors
    /// TODO
    fn get(&self, key: &K) -> Result<Option<V>, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn commit(self) -> Result<(), RuntimeError>;
}

//---------------------------------------------------------------------------------------------------- Read/Writer Transaction
/// TODO
///
/// Read/Write transaction.
pub trait RwTx<'env, K, V>: RoTx<'env, K, V> {
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

//---------------------------------------------------------------------------------------------------- IMPL

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

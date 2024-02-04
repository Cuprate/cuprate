//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::error::RuntimeError;

use std::path::Path;

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- TYPE
/// TODO
///
/// Database trait abstraction.
pub trait Database<'env>: Sized {
    //------------------------------------------------ Types
    /// Read-only transaction.
    type RoTx: 'env;

    /// Read/write transaction.
    type RwTx: 'env;

    //------------------------------------------------ Required
    /// TODO
    /// # Errors
    /// TODO
    fn open<P: AsRef<Path>>(path: P) -> Result<Self, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn sync(&self) -> Result<(), RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn tx_ro(&self) -> Result<Self::RoTx, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn tx_rw(&self) -> Result<Self::RwTx, RuntimeError>;

    //------------------------------------------------ Provided
}

//---------------------------------------------------------------------------------------------------- IMPL

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

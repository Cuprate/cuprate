//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::error::Error;

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
    fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error>;

    /// TODO
    /// # Errors
    /// TODO
    fn sync(&self) -> Result<(), Error>;

    /// TODO
    /// # Errors
    /// TODO
    fn tx_ro(&self) -> Result<Self::RoTx, Error>;

    /// TODO
    /// # Errors
    /// TODO
    fn tx_rw(&self) -> Result<Self::RwTx, Error>;

    //------------------------------------------------ Provided
}

//---------------------------------------------------------------------------------------------------- IMPL

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

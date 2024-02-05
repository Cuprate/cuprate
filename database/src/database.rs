//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    error::RuntimeError,
    table::Table,
    transaction::{RoTx, RwTx},
};

use std::path::Path;

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- TYPE
/// TODO
///
/// Database trait abstraction.
pub trait Database: Sized {
    //------------------------------------------------ Types
    /// TODO
    type RoTx<'db>;

    /// TODO
    type RwTx<'db>;

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
    fn tx_ro(&self) -> Result<Self::RoTx<'_>, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn tx_rw(&self) -> Result<Self::RwTx<'_>, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn create_table<T: Table>(
        &self,
        tx_rw: &mut Self::RwTx<'_>,
    ) -> Result<impl RwTx<'_, T::Key, T::Value>, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn get_table<T: Table>(
        &self,
        to_rw: &mut Self::RoTx<'_>,
    ) -> Result<Option<impl RoTx<'_, T::Key, T::Value>>, RuntimeError>;

    //------------------------------------------------ Provided
}

//---------------------------------------------------------------------------------------------------- IMPL

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

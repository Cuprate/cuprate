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
/// Database abstraction.
///
/// TODO: i'm pretty sure these lifetimes are unneeded/wrong.
pub trait Database: Sized {
    //------------------------------------------------ Types
    /// TODO
    type RoTx<'db>
    where
        Self: 'db;

    /// TODO
    type RwTx<'db>
    where
        Self: 'db;

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
    fn create_table<'db, T: Table + 'db>(
        &'db self,
        // tx_rw: &mut Self::RwTx<'_>,
    ) -> Result<impl RwTx<'db, T::Key, T::Value>, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn get_table<'db, T: Table + 'db>(
        &'db self,
        // to_rw: &mut Self::RoTx<'_>,
    ) -> Result<Option<impl RoTx<'db, T::Key, T::Value>>, RuntimeError>;

    //------------------------------------------------ Provided
}

//---------------------------------------------------------------------------------------------------- IMPL

//---------------------------------------------------------------------------------------------------- Trait Impl

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

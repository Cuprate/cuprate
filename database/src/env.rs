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
/// Database environment abstraction.
///
/// TODO: i'm pretty sure these lifetimes are unneeded/wrong.
pub trait Env: Sized {
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
    fn create_database<'db, T: Table + 'db>(
        &'db self,
        tx_rw: &'db mut Self::RwTx<'_>,
    ) -> Result<impl RwTx<'db, T::Key, T::Value>, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn open_database<'db, T: Table + 'db>(
        &'db self,
        to_rw: &'db Self::RoTx<'_>,
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

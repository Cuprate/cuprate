//! Abstracted database environment; `trait Env`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    database::Database,
    error::RuntimeError,
    table::Table,
    transaction::{RoTx, RwTx},
};

use std::path::Path;

//---------------------------------------------------------------------------------------------------- Env
/// Database environment abstraction.
///
/// TODO
pub trait Env: Sized {
    //------------------------------------------------ Types
    /// TODO
    type RoTx<'db>: RoTx<'db>;

    /// TODO
    type RwTx<'db>: RwTx<'db>;

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
    fn ro_tx(&self) -> Result<Self::RoTx<'_>, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn rw_tx(&self) -> Result<Self::RwTx<'_>, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn create_database<T: Table>(
        &self,
        tx_rw: &mut Self::RwTx<'_>,
    ) -> Result<impl Database<T>, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn open_database<T: Table>(
        &self,
        to_rw: &Self::RoTx<'_>,
    ) -> Result<Option<impl Database<T>>, RuntimeError>;

    //------------------------------------------------ Provided
}

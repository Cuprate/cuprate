//! Abstracted database environment; `trait Env`.

//---------------------------------------------------------------------------------------------------- Import
use std::path::Path;

use crate::{
    database::Database,
    error::RuntimeError,
    table::Table,
    transaction::{RoTx, RwTx},
};

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
    fn create_tables_if_needed<T: Table>(
        &self,
        rw_tx: &mut Self::RwTx<'_>,
    ) -> Result<(), RuntimeError>;

    /// TODO
    ///
    /// # TODO: Invariant
    /// This should never panic the database because the table doesn't exist.
    ///
    /// Opening/using the database env should have an invariant
    /// that it creates all the tables we need, such that this
    /// never returns `None`.
    ///
    /// # Errors
    /// TODO
    fn open_database<T: Table>(
        &self,
        ro_tx: &Self::RoTx<'_>,
    ) -> Result<impl Database<T>, RuntimeError>;

    //------------------------------------------------ Provided
}

//! Abstracted database environment; `trait Env`.

//---------------------------------------------------------------------------------------------------- Import
use std::path::Path;

use crate::{
    database::Database,
    error::{InitError, RuntimeError},
    table::Table,
    transaction::{RoTx, RwTx},
};

//---------------------------------------------------------------------------------------------------- Env
/// Database environment abstraction.
///
/// TODO
pub trait Env: Sized {
    //------------------------------------------------ Constants
    /// Does the database backend need to be manually
    /// resized when the memory-map is full?
    ///
    /// # Invariant
    /// If this is `false`, that means this [`Env`]
    /// can _never_ return a [`RuntimeError::ResizeNeeded`].
    ///
    /// If this is `true`, [`Env::resize_map`] & [`Env::current_map_size`]
    /// _must_ be re-implemented, as it just panics by default.
    const MANUAL_RESIZE: bool;

    //------------------------------------------------ Types
    /// TODO
    type RoTx<'db>: RoTx<'db>;

    /// TODO
    type RwTx<'db>: RwTx<'db>;

    //------------------------------------------------ Required
    /// TODO
    /// # Errors
    /// TODO
    fn open<P: AsRef<Path>>(path: P) -> Result<Self, InitError>;

    /// TODO
    /// # Errors
    /// TODO
    fn sync(&self) -> Result<(), RuntimeError>;

    /// Resize the database's memory map to a new size in bytes.
    ///
    /// # Errors
    /// TODO: should we panic or special case this?
    ///
    /// # Invariant
    /// This function _must_ be re-implemented if [`Env::MANUAL_RESIZE`] is `true`.
    ///
    /// Otherwise, this function will panic with `unreachable!()`.
    fn resize_map(new_size_bytes: usize) -> Result<(), RuntimeError> {
        unreachable!()
    }

    /// What is the _current_ size of the database's memory map in bytes?
    ///
    /// # Invariant
    /// 1. This function _must_ be re-implemented if [`Env::MANUAL_RESIZE`] is `true`.
    /// 2. This function must be accurate, as [`Env::resize()`] may depend on it.
    fn current_map_size() -> usize {
        unreachable!()
    }

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

//! Abstracted database environment; `trait Env`.

//---------------------------------------------------------------------------------------------------- Import
use std::path::Path;

#[allow(unused_imports)] // docs
use crate::ConcreteEnv;

use crate::{
    database::Database,
    error::{InitError, RuntimeError},
    table::Table,
    transaction::{RoTx, RwTx},
};

//---------------------------------------------------------------------------------------------------- Env
/// Database environment abstraction.
///
/// Essentially, the functions that can be called on [`ConcreteEnv`].
///
/// Objects that implement [`Env`] must be:
/// 1. Cheap to clone
/// 2. Thread safe
pub trait Env: Sized + Clone + Send + Sync + 'static {
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
    /// # Invariant
    /// This function _must_ be re-implemented if [`Env::MANUAL_RESIZE`] is `true`.
    ///
    /// Otherwise, this function will panic with `unreachable!()`.
    ///
    /// Database backend-specific invariants must also be upheld
    /// as this function will immediately resize.
    ///
    /// In particular for LMDB, this function should only be called
    /// if you have _mutual exclusive_ access to the database, i.e.
    /// there are no other readers or writers.
    /// <http://www.lmdb.tech/doc/group__mdb.html#gaa2506ec8dab3d969b0e609cd82e619e5>
    fn resize_map(&self, new_size_bytes: usize) {
        unreachable!()
    }

    /// What is the _current_ size of the database's memory map in bytes?
    ///
    /// # Invariant
    /// 1. This function _must_ be re-implemented if [`Env::MANUAL_RESIZE`] is `true`.
    /// 2. This function must be accurate, as [`Env::resize_map()`] may depend on it.
    fn current_map_size(&self) -> usize {
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

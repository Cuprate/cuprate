//! Abstracted database environment; `trait Env`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    config::Config,
    database::Database,
    error::{InitError, RuntimeError},
    resize::ResizeAlgorithm,
    table::Table,
    transaction::{RoTx, RwTx},
};

//---------------------------------------------------------------------------------------------------- Env
/// Database environment abstraction.
///
/// Essentially, the functions that can be called on [`ConcreteEnv`](crate::ConcreteEnv).
///
/// # `Drop`
/// Objects that implement [`Env`] _should_ probably
/// [`Env::sync`] in their drop implementations,
/// although, no invariant relies on this (yet).
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

    /// Does the database backend forcefully sync/flush
    /// to disk on every transaction commit?
    ///
    /// This is used as an optimization.
    const SYNCS_PER_TX: bool;

    //------------------------------------------------ Types
    /// TODO
    type RoTx<'db>: RoTx<'db>;

    /// TODO
    type RwTx<'db>: RwTx<'db>;

    //------------------------------------------------ Required
    /// TODO
    /// # Errors
    /// TODO
    fn open(config: Config) -> Result<Self, InitError>;

    /// Return the [`Config`] that this database was [`Env::open`]ed with.
    fn config(&self) -> &Config;

    /// Return the amount of actual of bytes the database is taking up on disk.
    ///
    /// This is the current _disk_ value in bytes, not the memory map.
    ///
    /// # Errors
    /// This will error if either:
    ///
    /// - [`std::fs::File::open`]
    /// - [`std::fs::File::metadata`]
    ///
    /// failed on the database file on disk.
    fn disk_size_bytes(&self) -> std::io::Result<u64> {
        // We have the direct PATH to the file,
        // no need to use backend-specific functions.
        //
        // SAFETY: as we are only accessing the metadata of
        // the file and not reading the bytes, it should be
        // fine even with a memory mapped file being actively
        // written to.
        Ok(std::fs::File::open(&self.config().db_file)?
            .metadata()?
            .len())
    }

    /// TODO
    /// # Errors
    /// TODO
    fn sync(&self) -> Result<(), RuntimeError>;

    /// Resize the database's memory map to a
    /// new (bigger) size using a [`ResizeAlgorithm`].
    ///
    /// By default, this function will use the `ResizeAlgorithm` in [`Env::config`].
    ///
    /// If `resize_algorithm` is `Some`, that will be used instead.
    ///
    /// # Invariant
    /// This function _must_ be re-implemented if [`Env::MANUAL_RESIZE`] is `true`.
    ///
    /// Otherwise, this function will panic with `unreachable!()`.
    fn resize_map(&self, resize_algorithm: Option<ResizeAlgorithm>) {
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

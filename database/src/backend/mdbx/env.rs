//! Implementation of `trait Env` for `mdbx`.

//---------------------------------------------------------------------------------------------------- Import
use std::{path::Path, sync::Arc};

use crate::{
    backend::mdbx::types::MdbxDb,
    config::Config,
    database::Database,
    env::Env,
    error::{InitError, RuntimeError},
    table::Table,
};

//---------------------------------------------------------------------------------------------------- ConcreteEnv
/// A strongly typed, concrete database environment, backed by `libmdbx`.
pub struct ConcreteEnv {
    /// The actual database environment.
    ///
    /// # `WriteMap` usage
    /// Reference: <https://erthink.github.io/libmdbx/intro.html>.
    env: libmdbx::Database<libmdbx::WriteMap>,

    /// The configuration we were opened with
    /// (and in current use).
    config: Config,
}

impl Drop for ConcreteEnv {
    fn drop(&mut self) {
        if let Err(e) = self.sync() {
            // TODO: log error?
        }

        // TODO: log that we are dropping the database.
    }
}

//---------------------------------------------------------------------------------------------------- Env Impl
impl Env for ConcreteEnv {
    // MDBX resizes automatically, with customizable settings:
    /// <https://erthink.github.io/libmdbx/group__c__settings.html#ga79065e4f3c5fb2ad37a52b59224d583e>.
    const MANUAL_RESIZE: bool = false;
    const SYNCS_PER_TX: bool = false;
    type RoTx<'db> = libmdbx::Transaction<'db, libmdbx::RO, libmdbx::WriteMap>;
    type RwTx<'db> = libmdbx::Transaction<'db, libmdbx::RW, libmdbx::WriteMap>;

    #[cold]
    #[inline(never)] // called once.
    fn open(config: Config) -> Result<Self, InitError> {
        todo!()
    }

    fn config(&self) -> &Config {
        &self.config
    }

    fn sync(&self) -> Result<(), RuntimeError> {
        todo!()
    }

    #[inline]
    fn ro_tx(&self) -> Result<Self::RoTx<'_>, RuntimeError> {
        todo!()
    }

    #[inline]
    fn rw_tx(&self) -> Result<Self::RwTx<'_>, RuntimeError> {
        todo!()
    }

    #[cold]
    #[inline(never)] // called infrequently?.
    fn create_tables_if_needed<T: Table>(
        &self,
        tx_rw: &mut Self::RwTx<'_>,
    ) -> Result<(), RuntimeError> {
        todo!()
    }

    #[inline]
    fn open_database<T: Table>(
        &self,
        to_rw: &Self::RoTx<'_>,
    ) -> Result<impl Database<T>, RuntimeError> {
        let tx: MdbxDb = todo!();
        Ok(tx)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

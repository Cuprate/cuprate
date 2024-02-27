//! Implementation of `trait Env` for `redb`.

//---------------------------------------------------------------------------------------------------- Import
use std::{path::Path, sync::Arc};

use crate::{
    backend::redb::types::{RedbTableRo, RedbTableRw},
    config::Config,
    database::{DatabaseRo, DatabaseRw},
    env::Env,
    error::{InitError, RuntimeError},
    table::Table,
};

//---------------------------------------------------------------------------------------------------- ConcreteEnv
/// A strongly typed, concrete database environment, backed by `redb`.
pub struct ConcreteEnv {
    /// The actual database environment.
    env: redb::Database,

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
    const MANUAL_RESIZE: bool = false;
    const SYNCS_PER_TX: bool = false;

    type TxRo<'db> = redb::ReadTransaction<'db>;
    type TxRw<'db> = redb::WriteTransaction<'db>;

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
    fn tx_ro(&self) -> Result<Self::TxRo<'_>, RuntimeError> {
        todo!()
    }

    #[inline]
    fn tx_rw(&self) -> Result<Self::TxRw<'_>, RuntimeError> {
        todo!()
    }

    #[cold]
    #[inline(never)] // called infrequently?.
    fn create_tables_if_needed<T: Table>(
        &self,
        tx_rw: &mut Self::TxRw<'_>,
    ) -> Result<(), RuntimeError> {
        todo!()
    }

    #[inline]
    fn open_db_ro<T: Table>(
        &self,
        tx_ro: &Self::TxRo<'_>,
    ) -> Result<impl DatabaseRo<T>, RuntimeError> {
        let tx: RedbTableRo = todo!();
        Ok(tx)
    }

    #[inline]
    fn open_db_rw<T: Table>(
        &self,
        tx_rw: &mut Self::TxRw<'_>,
    ) -> Result<impl DatabaseRw<T>, RuntimeError> {
        let tx: RedbTableRw = todo!();
        Ok(tx)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

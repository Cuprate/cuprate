//! Implementation of `trait Env` for `redb`.

//---------------------------------------------------------------------------------------------------- Import
use std::{path::Path, sync::Arc};

use crate::{
    backend::redb::types::{RedbTableRo, RedbTableRw},
    config::{Config, SyncMode},
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

    /// A cached, redb version of `cuprate_database::config::SyncMode`.
    /// `redb` needs the sync mode to be set _per_ TX, so we
    /// will continue to use this value every `Env::tx_rw`.
    durability: redb::Durability,
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

    type TxRo<'env> = redb::ReadTransaction<'env>;
    type TxRw<'env> = redb::WriteTransaction<'env>;

    #[cold]
    #[inline(never)] // called once.
    fn open(config: Config) -> Result<Self, InitError> {
        // TODO: dynamic syncs are not implemented.
        let durability = match config.sync_mode {
            // TODO: There's also `redb::Durability::Paranoid`:
            // <https://docs.rs/redb/1.5.0/redb/enum.Durability.html#variant.Paranoid>
            // should we use that instead of Immediate?
            SyncMode::Safe => redb::Durability::Immediate,
            SyncMode::Async => redb::Durability::Eventual,
            SyncMode::Fast => redb::Durability::None,
            // TODO: dynamic syncs are not implemented.
            SyncMode::FastThenSafe | SyncMode::Threshold(_) => unimplemented!(),
        };

        todo!()
    }

    #[cold]
    #[inline(never)] // called once in [`Env::open`]?`
    fn create_tables<T: Table>(&self, tx_rw: &mut Self::TxRw<'_>) -> Result<(), RuntimeError> {
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
        // `redb` has sync modes on the TX level, unlike heed,
        // which sets it at the Environment level.
        //
        // So, set the durability here before returning the TX.
        let mut tx_rw = self.env.begin_write()?;
        tx_rw.set_durability(self.durability);
        Ok(tx_rw)
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

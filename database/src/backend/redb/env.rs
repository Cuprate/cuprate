//! Implementation of `trait Env` for `redb`.

//---------------------------------------------------------------------------------------------------- Import
use std::{ops::Deref, path::Path, sync::Arc};

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
        // INVARIANT: drop(ConcreteEnv) must sync.
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
    type EnvInner = redb::Database;
    type TxCreator = Self;

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

    fn config(&self) -> &Config {
        &self.config
    }

    fn sync(&self) -> Result<(), RuntimeError> {
        todo!()
    }

    #[inline]
    fn tx_creator(&self) -> impl Deref<Target = Self::TxCreator<'_>> {
        self
    }

    #[inline]
    fn tx_rw_input(&self) -> impl Deref<Target = Self::TxRwInput> {
        self
    }

    #[inline]
    fn tx_ro(env: &Self::TxRoInput) -> Result<Self::TxRo<'_>, RuntimeError> {
        Ok(env.begin_read()?)
    }

    #[inline]
    fn tx_rw(this: &Self::TxRwInput) -> Result<Self::TxRw<'_>, RuntimeError> {
        // `redb` has sync modes on the TX level, unlike heed,
        // which sets it at the Environment level.
        //
        // So, set the durability here before returning the TX.
        let mut tx_rw = this.env.begin_write()?;
        tx_rw.set_durability(this.durability);
        Ok(tx_rw)
    }

    #[inline]
    fn open_db_ro<'tx, T: Table>(
        env: &Self::EnvInner,
        tx_ro: &'tx Self::TxRo<'tx>,
    ) -> Result<impl DatabaseRo<'tx, T>, RuntimeError> {
        let tx: RedbTableRo<'tx, T::Key, T::Value> = todo!();
        Ok(tx)
    }

    #[inline]
    fn open_db_rw<'tx, T: Table>(
        env: &Self::EnvInner,
        tx_rw: &'tx mut Self::TxRw<'tx>,
    ) -> Result<impl DatabaseRw<'tx, T>, RuntimeError> {
        let tx: RedbTableRw<'tx, 'tx, T::Key, T::Value> = todo!();
        Ok(tx)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

//! Implementation of `trait Env` for `redb`.

//---------------------------------------------------------------------------------------------------- Import
use std::{ops::Deref, path::Path, sync::Arc};

use crate::{
    backend::redb::types::{RedbTableRo, RedbTableRw},
    config::{Config, SyncMode},
    database::{DatabaseRo, DatabaseRw},
    env::{Env, EnvInner},
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
    type EnvInner<'env> = (&'env redb::Database, redb::Durability);
    type TxRo<'tx> = redb::ReadTransaction<'tx>;
    type TxRw<'tx> = redb::WriteTransaction<'tx>;

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

    fn env_inner(&self) -> Self::EnvInner<'_> {
        (&self.env, self.durability)
    }
}

//---------------------------------------------------------------------------------------------------- EnvInner Impl
impl<'env> EnvInner<'env, redb::ReadTransaction<'env>, redb::WriteTransaction<'env>>
    for (&'env redb::Database, redb::Durability)
where
    Self: 'env,
{
    #[inline]
    fn tx_ro(&'env self) -> Result<redb::ReadTransaction<'env>, RuntimeError> {
        Ok(self.0.begin_read()?)
    }

    #[inline]
    fn tx_rw(&'env self) -> Result<redb::WriteTransaction<'env>, RuntimeError> {
        // `redb` has sync modes on the TX level, unlike heed,
        // which sets it at the Environment level.
        //
        // So, set the durability here before returning the TX.
        let mut tx_rw = self.0.begin_write()?;
        tx_rw.set_durability(self.1);
        Ok(tx_rw)
    }

    #[inline]
    fn open_db_ro<'tx, T: Table>(
        &self,
        tx_ro: &'tx redb::ReadTransaction<'env>,
    ) -> Result<impl DatabaseRo<'tx, T>, RuntimeError> {
        let tx: RedbTableRo<'tx, T::Key, T::Value> = todo!();
        Ok(tx)
    }

    #[inline]
    fn open_db_rw<'tx, T: Table>(
        &self,
        tx_ro: &'tx mut redb::WriteTransaction<'env>,
    ) -> Result<impl DatabaseRw<'env, 'tx, T>, RuntimeError> {
        let tx: RedbTableRw<'env, 'tx, T::Key, T::Value> = todo!();
        Ok(tx)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

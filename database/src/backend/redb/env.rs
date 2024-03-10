//! Implementation of `trait Env` for `redb`.

//---------------------------------------------------------------------------------------------------- Import
use std::{ops::Deref, path::Path, sync::Arc};

use crate::{
    backend::redb::{
        storable::StorableRedb,
        types::{RedbTableRo, RedbTableRw},
    },
    config::{Config, SyncMode},
    database::{DatabaseRo, DatabaseRw},
    env::{Env, EnvInner},
    error::{InitError, RuntimeError},
    table::Table,
    TxRw,
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
    #[allow(clippy::items_after_statements)]
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

        let env_builder = redb::Builder::new();

        // TODO: we can set cache sizes with:
        // env_builder.set_cache(bytes);

        // Open the database file, create if needed.
        let db_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(config.db_file())?;
        let mut env = env_builder.create_file(db_file)?;

        // Create all database tables.
        // `redb` creates tables if they don't exist.
        // <https://docs.rs/redb/latest/redb/struct.WriteTransaction.html#method.open_table>
        use crate::tables::{TestTable, TestTable2};
        let tx_rw = env.begin_write()?;

        // FIXME:
        // These wonderful fully qualified trait types are brought
        // to you by `tower::discover::Discover>::Key` collisions.

        // TestTable
        let table: redb::TableDefinition<
            'static,
            StorableRedb<<TestTable as Table>::Key>,
            StorableRedb<<TestTable as Table>::Value>,
        > = redb::TableDefinition::new(TestTable::NAME);
        tx_rw.open_table(table)?;

        // TestTable2
        let table: redb::TableDefinition<
            'static,
            StorableRedb<<TestTable2 as Table>::Key>,
            StorableRedb<<TestTable2 as Table>::Value>,
        > = redb::TableDefinition::new(TestTable2::NAME);
        tx_rw.open_table(table)?;

        tx_rw.commit()?;

        // Check for file integrity.
        // TODO: should we do this? is it slow?
        env.check_integrity()?;

        Ok(Self {
            env,
            config,
            durability,
        })
    }

    fn config(&self) -> &Config {
        &self.config
    }

    fn sync(&self) -> Result<(), RuntimeError> {
        // `redb`'s syncs are tied with write transactions,
        // so just create one, don't do anything and commit.
        let mut tx_rw = self.env.begin_write()?;
        tx_rw.set_durability(redb::Durability::Paranoid);
        TxRw::commit(tx_rw)
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
        // Open up a read-only database using our `T: Table`'s const metadata.
        let table: redb::TableDefinition<'static, StorableRedb<T::Key>, StorableRedb<T::Value>> =
            redb::TableDefinition::new(T::NAME);

        // INVARIANT: Our `?` error conversion will panic if the table does not exist.
        Ok(tx_ro.open_table(table)?)
    }

    #[inline]
    fn open_db_rw<'tx, T: Table>(
        &self,
        tx_rw: &'tx mut redb::WriteTransaction<'env>,
    ) -> Result<impl DatabaseRw<'env, 'tx, T>, RuntimeError> {
        // Open up a read/write database using our `T: Table`'s const metadata.
        let table: redb::TableDefinition<'static, StorableRedb<T::Key>, StorableRedb<T::Value>> =
            redb::TableDefinition::new(T::NAME);

        // `redb` creates tables if they don't exist, so this should never panic.
        // <https://docs.rs/redb/latest/redb/struct.WriteTransaction.html#method.open_table>
        Ok(tx_rw.open_table(table)?)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

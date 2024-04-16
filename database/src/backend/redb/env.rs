//! Implementation of `trait Env` for `redb`.

//---------------------------------------------------------------------------------------------------- Import
use std::{fmt::Debug, ops::Deref, path::Path, sync::Arc};

use crate::{
    backend::redb::{
        storable::StorableRedb,
        types::{RedbTableRo, RedbTableRw},
    },
    config::{Config, SyncMode},
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
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
    type TxRo<'tx> = redb::ReadTransaction;
    type TxRw<'tx> = redb::WriteTransaction;

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

        // Use the in-memory backend if the feature is enabled.
        let mut env = if cfg!(feature = "redb-memory") {
            env_builder.create_with_backend(redb::backends::InMemoryBackend::new())?
        } else {
            // Create the database directory if it doesn't exist.
            std::fs::create_dir_all(config.db_directory())?;

            // Open the database file, create if needed.
            let db_file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(config.db_file())?;

            env_builder.create_file(db_file)?
        };

        // Create all database tables.
        // `redb` creates tables if they don't exist.
        // <https://docs.rs/redb/latest/redb/struct.WriteTransaction.html#method.open_table>

        /// Function that creates the tables based off the passed `T: Table`.
        fn create_table<T: Table>(tx_rw: &redb::WriteTransaction) -> Result<(), InitError> {
            // TODO: use tracing.
            // println!("create_table(): {}", T::NAME);

            let table: redb::TableDefinition<
                'static,
                StorableRedb<<T as Table>::Key>,
                StorableRedb<<T as Table>::Value>,
            > = redb::TableDefinition::new(<T as Table>::NAME);

            // `redb` creates tables on open if not already created.
            tx_rw.open_table(table)?;
            Ok(())
        }

        use crate::tables::{
            BlockBlobs, BlockHeights, BlockInfoV1s, BlockInfoV2s, BlockInfoV3s, KeyImages,
            NumOutputs, Outputs, PrunableHashes, PrunableTxBlobs, PrunedTxBlobs, RctOutputs,
            TxHeights, TxIds, TxUnlockTime,
        };

        let tx_rw = env.begin_write()?;
        create_table::<BlockBlobs>(&tx_rw)?;
        create_table::<BlockHeights>(&tx_rw)?;
        create_table::<BlockInfoV1s>(&tx_rw)?;
        create_table::<BlockInfoV2s>(&tx_rw)?;
        create_table::<BlockInfoV3s>(&tx_rw)?;
        create_table::<KeyImages>(&tx_rw)?;
        create_table::<NumOutputs>(&tx_rw)?;
        create_table::<Outputs>(&tx_rw)?;
        create_table::<PrunableHashes>(&tx_rw)?;
        create_table::<PrunableTxBlobs>(&tx_rw)?;
        create_table::<PrunedTxBlobs>(&tx_rw)?;
        create_table::<RctOutputs>(&tx_rw)?;
        create_table::<TxHeights>(&tx_rw)?;
        create_table::<TxIds>(&tx_rw)?;
        create_table::<TxUnlockTime>(&tx_rw)?;
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
impl<'env> EnvInner<'env, redb::ReadTransaction, redb::WriteTransaction>
    for (&'env redb::Database, redb::Durability)
where
    Self: 'env,
{
    #[inline]
    fn tx_ro(&'env self) -> Result<redb::ReadTransaction, RuntimeError> {
        Ok(self.0.begin_read()?)
    }

    #[inline]
    fn tx_rw(&'env self) -> Result<redb::WriteTransaction, RuntimeError> {
        // `redb` has sync modes on the TX level, unlike heed,
        // which sets it at the Environment level.
        //
        // So, set the durability here before returning the TX.
        let mut tx_rw = self.0.begin_write()?;
        tx_rw.set_durability(self.1);
        Ok(tx_rw)
    }

    #[inline]
    fn open_db_ro<T: Table>(
        &self,
        tx_ro: &redb::ReadTransaction,
    ) -> Result<impl DatabaseRo<T> + DatabaseIter<T>, RuntimeError> {
        // Open up a read-only database using our `T: Table`'s const metadata.
        let table: redb::TableDefinition<'static, StorableRedb<T::Key>, StorableRedb<T::Value>> =
            redb::TableDefinition::new(T::NAME);

        // INVARIANT: Our `?` error conversion will panic if the table does not exist.
        Ok(tx_ro.open_table(table)?)
    }

    #[inline]
    fn open_db_rw<T: Table>(
        &self,
        tx_rw: &redb::WriteTransaction,
    ) -> Result<impl DatabaseRw<T>, RuntimeError> {
        // Open up a read/write database using our `T: Table`'s const metadata.
        let table: redb::TableDefinition<'static, StorableRedb<T::Key>, StorableRedb<T::Value>> =
            redb::TableDefinition::new(T::NAME);

        // `redb` creates tables if they don't exist, so this should never panic.
        // <https://docs.rs/redb/latest/redb/struct.WriteTransaction.html#method.open_table>
        Ok(tx_rw.open_table(table)?)
    }

    #[inline]
    fn clear_db<T: Table>(&self, tx_rw: &mut redb::WriteTransaction) -> Result<(), RuntimeError> {
        let table: redb::TableDefinition<
            'static,
            StorableRedb<<T as Table>::Key>,
            StorableRedb<<T as Table>::Value>,
        > = redb::TableDefinition::new(<T as Table>::NAME);

        // INVARIANT:
        // This `delete_table()` will not run into this `TableAlreadyOpen` error:
        // <https://docs.rs/redb/2.0.0/src/redb/transactions.rs.html#382>
        // which will panic in the `From` impl, as:
        //
        // 1. Only 1 `redb::WriteTransaction` can exist at a time
        // 2. We have exclusive access to it
        // 3. So it's not being used to open a table since that needs `&tx_rw`
        //
        // Reader-open tables do not affect this, if they're open the below is still OK.
        redb::WriteTransaction::delete_table(tx_rw, table)?;
        // Re-create the table.
        // `redb` creates tables if they don't exist, so this should never panic.
        redb::WriteTransaction::open_table(tx_rw, table)?;

        Ok(())
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

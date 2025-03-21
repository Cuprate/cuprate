//! Implementation of `trait Env` for `redb`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    backend::redb::storable::StorableRedb,
    config::{Config, SyncMode},
    database::{DatabaseIter, DatabaseRo, DatabaseRw},
    env::{Env, EnvInner},
    error::{DbResult, InitError, RuntimeError},
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
            #[cfg(feature = "tracing")]
            warn!("Env sync error: {e}");
        }
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
    fn open(config: Config) -> Result<Self, InitError> {
        // SOMEDAY: dynamic syncs are not implemented.
        let durability = match config.sync_mode {
            SyncMode::Safe => redb::Durability::Immediate,
            // TODO: impl `FastThenSafe`
            SyncMode::FastThenSafe | SyncMode::Fast => redb::Durability::Eventual,
        };

        let env_builder = redb::Builder::new();

        // FIXME: we can set cache sizes with:
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
                .truncate(false)
                .open(config.db_file())?;

            env_builder.create_file(db_file)?
        };

        // Create all database tables.
        // `redb` creates tables if they don't exist.
        // <https://docs.rs/redb/latest/redb/struct.WriteTransaction.html#method.open_table>

        // Check for file integrity.
        // FIXME: should we do this? is it slow?
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

    fn sync(&self) -> DbResult<()> {
        // `redb`'s syncs are tied with write transactions,
        // so just create one, don't do anything and commit.
        let mut tx_rw = self.env.begin_write()?;
        tx_rw.set_durability(redb::Durability::Immediate);
        tx_rw.set_two_phase_commit(true);
        TxRw::commit(tx_rw)
    }

    fn env_inner(&self) -> Self::EnvInner<'_> {
        (&self.env, self.durability)
    }
}

//---------------------------------------------------------------------------------------------------- EnvInner Impl
impl<'env> EnvInner<'env> for (&'env redb::Database, redb::Durability)
where
    Self: 'env,
{
    type Ro<'a> = redb::ReadTransaction;
    type Rw<'a> = redb::WriteTransaction;

    #[inline]
    fn tx_ro(&self) -> DbResult<redb::ReadTransaction> {
        Ok(self.0.begin_read()?)
    }

    #[inline]
    fn tx_rw(&self) -> DbResult<redb::WriteTransaction> {
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
        tx_ro: &Self::Ro<'_>,
    ) -> DbResult<impl DatabaseRo<T> + DatabaseIter<T>> {
        // Open up a read-only database using our `T: Table`'s const metadata.
        let table: redb::TableDefinition<'static, StorableRedb<T::Key>, StorableRedb<T::Value>> =
            redb::TableDefinition::new(T::NAME);

        Ok(tx_ro.open_table(table)?)
    }

    #[inline]
    fn open_db_rw<T: Table>(&self, tx_rw: &Self::Rw<'_>) -> DbResult<impl DatabaseRw<T>> {
        // Open up a read/write database using our `T: Table`'s const metadata.
        let table: redb::TableDefinition<'static, StorableRedb<T::Key>, StorableRedb<T::Value>> =
            redb::TableDefinition::new(T::NAME);

        // `redb` creates tables if they don't exist, so this shouldn't return `RuntimeError::TableNotFound`.
        // <https://docs.rs/redb/latest/redb/struct.WriteTransaction.html#method.open_table>
        Ok(tx_rw.open_table(table)?)
    }

    fn create_db<T: Table>(&self, tx_rw: &redb::WriteTransaction) -> DbResult<()> {
        // INVARIANT: `redb` creates tables if they don't exist.
        self.open_db_rw::<T>(tx_rw)?;
        Ok(())
    }

    #[inline]
    fn clear_db<T: Table>(&self, tx_rw: &mut redb::WriteTransaction) -> DbResult<()> {
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
        if !redb::WriteTransaction::delete_table(tx_rw, table)? {
            return Err(RuntimeError::TableNotFound);
        }

        // Re-create the table.
        // `redb` creates tables if they don't exist, so this should never panic.
        redb::WriteTransaction::open_table(tx_rw, table)?;

        Ok(())
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod tests {}

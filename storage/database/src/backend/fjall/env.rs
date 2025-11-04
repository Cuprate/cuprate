use std::cell::RefCell;
use fjall::{PersistMode, compaction, TransactionalKeyspace, WriteTransaction, ReadTransaction, PartitionCreateOptions, CompressionType};
use fjall::compaction::{SizeTiered, Strategy};
use crate::config::{Config, SyncMode};
use crate::{DatabaseIter, DatabaseRo, DatabaseRw, DbResult, Env, EnvInner, InitError, RuntimeError, Table};
use crate::backend::fjall::database::{FjallTableRo, FjallTableRw};

pub type ConcreteEnv = (TransactionalKeyspace, Config);

impl Env for (TransactionalKeyspace, Config) {
    const MANUAL_RESIZE: bool = false;
    const SYNCS_PER_TX: bool = false;
    type EnvInner<'env>
    where
        Self: 'env
    = &'env Self;
    type TxRo<'env>
    where
        Self: 'env
    = ReadTransaction;
    type TxRw<'env>
    where
        Self: 'env
    = RefCell<WriteTransaction<'env>>;

    fn open(config: Config) -> Result<Self, InitError> {
        Ok((fjall::Config::new(config.db_file()).max_write_buffer_size(1 * 1024 * 1024 * 1024).manual_journal_persist(true).open_transactional()?, config))
    }

    fn config(&self) -> &Config {
        &self.1
    }

    fn sync(&self) -> DbResult<()> {
        self.0.persist(PersistMode::SyncData)?;
        Ok(())
    }

    fn env_inner(&self) -> Self::EnvInner<'_> {
        self
    }
}

impl EnvInner<'_> for  &'_ (TransactionalKeyspace, Config) {
    type Ro<'tx> = ReadTransaction;
    type Rw<'tx> = RefCell<WriteTransaction<'tx>>;

    fn tx_ro(&self) -> DbResult<Self::Ro<'_>> {
        Ok(self.0.read_tx())
    }

    fn tx_rw(&self) -> DbResult<Self::Rw<'_>> {
        Ok(RefCell::new(self.0.write_tx().durability(None)))
    }

    fn open_db_ro<T: Table>(&self, tx_ro: &Self::Ro<'_>) -> DbResult<impl DatabaseRo<T> + DatabaseIter<T>> {
        if !self.0.partition_exists(T::NAME) {
            return Err(RuntimeError::TableNotFound)
        }

        let handle = self.0.open_partition(T::NAME, PartitionCreateOptions::default().max_memtable_size(64 * 1024 *1024).manual_journal_persist(true).compression(CompressionType::None))?;
        Ok(FjallTableRo {
            handle,
            read_tx: tx_ro
        })
    }

    fn open_db_rw<T: Table>(&self, tx_rw: &Self::Rw<'_>) -> DbResult<impl DatabaseRw<T>> {
        let handle = self.0.open_partition(T::NAME, PartitionCreateOptions::default().max_memtable_size(64 * 1024 *1024).manual_journal_persist(true).compression(CompressionType::None))?;

        Ok(FjallTableRw {
            handle,
            read_tx: tx_rw,
        })
    }

    fn create_db<T: Table>(&self, tx_rw: &Self::Rw<'_>) -> DbResult<()> {
        let handle = self.0.open_partition(T::NAME, PartitionCreateOptions::default().max_memtable_size(64 * 1024 *1024).manual_journal_persist(true).compression(CompressionType::None))?;
        Ok(())
    }

    fn clear_db<T: Table>(&self, tx_rw: &mut Self::Rw<'_>) -> DbResult<()> {
        let handle = self.0.open_partition(T::NAME, PartitionCreateOptions::default())?;

        self.0.delete_partition(handle)?;
        Ok(())
    }
}
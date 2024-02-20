//! Implementation of `trait Env` for `heed`.

//---------------------------------------------------------------------------------------------------- Import
use std::sync::RwLock;

use crate::{
    backend::heed::types::HeedDb,
    config::Config,
    database::Database,
    env::Env,
    error::{InitError, RuntimeError},
    table::Table,
};

//---------------------------------------------------------------------------------------------------- Env
/// A strongly typed, concrete database environment, backed by `heed`.
///
/// # Why `RwLock`?
/// We need mutual exclusive access to the environment for resizing.
pub struct ConcreteEnv(RwLock<heed::Env>);

impl Drop for ConcreteEnv {
    fn drop(&mut self) {
        if let Err(e) = self.sync() {
            // TODO: log error?
        }
    }
}

//---------------------------------------------------------------------------------------------------- Env Impl
impl Env for ConcreteEnv {
    const MANUAL_RESIZE: bool = true;
    const SYNCS_PER_TX: bool = false;
    type RoTx<'db> = heed::RoTxn<'db>;
    type RwTx<'db> = heed::RwTxn<'db>;

    #[cold]
    #[inline(never)] // called once.
    fn open(config: Config) -> Result<Self, InitError> {
        // INVARIANT:
        // We must open LMDB using `heed::EnvOpenOptions::max_readers`
        // and input whatever is in `config.reader_threads` or else
        // LMDB will start throwing errors if there are >126 readers.
        // <http://www.lmdb.tech/doc/group__mdb.html#gae687966c24b790630be2a41573fe40e2>

        todo!()
    }

    fn config(&self) -> &Config {
        todo!()
    }

    fn sync(&self) -> Result<(), RuntimeError> {
        todo!()
    }

    fn resize_map(&self, new_size: usize) {
        // INVARIANT: Resizing requires that we have
        // exclusive access to the database environment.
        // hang until all readers have exited.
        #[allow(clippy::readonly_write_lock)]
        let _env_lock_guard = self.0.write().unwrap();

        todo!()
    }

    fn current_map_size(&self) -> usize {
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
        let tx: HeedDb = todo!();
        Ok(tx)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

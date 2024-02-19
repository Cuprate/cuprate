//! Implementation of `trait Env` for `heed`.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    path::Path,
    sync::{Arc, RwLock},
};

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
/// # Why `Arc<RwLock>`?
/// TLDR: We need mutual exclusive access to the environment for resizing.
///
/// Initially, I though to separate the lock and `heed::Env` as it already
/// uses `Arc` internally, and wrapping it again in `Arc` seemed... wrong,
/// but the other field would be `Arc<RwLock<()>>` since this structure
/// needs to be cheaply clonable.
///
/// In the end, we have to deref 2 `Arc`s anyway, so it's the same.
#[derive(Clone)]
pub struct ConcreteEnv(Arc<RwLock<heed::Env>>);

//---------------------------------------------------------------------------------------------------- Env Impl
impl Env for ConcreteEnv {
    const MANUAL_RESIZE: bool = true;
    const SYNCS_PER_TX: bool = false;
    type RoTx<'db> = heed::RoTxn<'db>;
    type RwTx<'db> = heed::RwTxn<'db>;

    #[cold]
    #[inline(never)] // called once.
    fn open<P: AsRef<Path>>(path: P, config: Config) -> Result<Self, InitError> {
        todo!()
    }

    fn path(&self) -> &Path {
        todo!()
    }

    fn sync(&self) -> Result<(), RuntimeError> {
        todo!()
    }

    fn resize_map(&self, new_size: usize) {
        // INVARIANT: Resizing requires that we have
        // exclusive access to the database environment.
        // hang until all readers have exited.
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

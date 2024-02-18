//! Implementation of `trait Env` for `heed`.

//---------------------------------------------------------------------------------------------------- Import
use std::path::Path;

use crate::{
    backend::heed::types::HeedDb,
    database::Database,
    env::Env,
    error::{InitError, RuntimeError},
    table::Table,
};

//---------------------------------------------------------------------------------------------------- Env
/// A strongly typed, concrete database environment, backed by `heed`.
#[derive(Clone)]
// No need for `Arc`, `heed::Env` already uses it internally and implements `Clone`.
pub struct ConcreteEnv(heed::Env);

//---------------------------------------------------------------------------------------------------- Env Impl
impl Env for ConcreteEnv {
    const MANUAL_RESIZE: bool = true;
    const SYNCS_PER_TX: bool = false;
    type RoTx<'db> = heed::RoTxn<'db>;
    type RwTx<'db> = heed::RwTxn<'db>;

    #[cold]
    #[inline(never)] // called once.
    fn open<P: AsRef<Path>>(path: P, sync_per_tx: bool) -> Result<Self, InitError> {
        todo!()
    }

    fn path(&self) -> &Path {
        todo!()
    }

    fn sync(&self) -> Result<(), RuntimeError> {
        todo!()
    }

    fn resize_map(&self, new_size: usize) {
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

//! Implementation of `trait Env` for `sanakirja`.

//---------------------------------------------------------------------------------------------------- Import
use std::{path::Path, sync::Arc};

use crate::{
    backend::sanakirja::types::SanakirjaDb,
    config::Config,
    database::Database,
    env::Env,
    error::{InitError, RuntimeError},
    table::Table,
};

//---------------------------------------------------------------------------------------------------- ConcreteEnv
/// A strongly typed, concrete database environment, backed by `sanakirja`.
pub struct ConcreteEnv(sanakirja::Env);

impl Drop for ConcreteEnv {
    fn drop(&mut self) {
        if let Err(e) = self.sync() {
            // TODO: log error?
        }
    }
}

//---------------------------------------------------------------------------------------------------- Env Impl
impl Env for ConcreteEnv {
    const MANUAL_RESIZE: bool = false;
    const SYNCS_PER_TX: bool = true;
    /// FIXME:
    /// We could also implement `Borrow<sanakirja::Env> for ConcreteEnv`
    /// instead of this reference.
    type RoTx<'db> = sanakirja::Txn<&'db sanakirja::Env>;
    type RwTx<'db> = sanakirja::MutTxn<&'db sanakirja::Env, ()>;

    #[cold]
    #[inline(never)] // called once.
    fn open(config: Config) -> Result<Self, InitError> {
        todo!()
    }

    fn config(&self) -> &Config {
        todo!()
    }

    fn sync(&self) -> Result<(), RuntimeError> {
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
        let tx: SanakirjaDb = todo!();
        Ok(tx)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

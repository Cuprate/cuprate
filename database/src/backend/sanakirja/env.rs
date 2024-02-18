//! Implementation of `trait Env` for `sanakirja`.

//---------------------------------------------------------------------------------------------------- Import
use std::path::Path;

use crate::{
    backend::sanakirja::types::SanakirjaDb,
    database::Database,
    env::Env,
    error::{InitError, RuntimeError},
    table::Table,
};

//---------------------------------------------------------------------------------------------------- ConcreteEnv
/// A strongly typed, concrete database environment, backed by `sanakirja`.
pub struct ConcreteEnv(sanakirja::Env);

//---------------------------------------------------------------------------------------------------- Env Impl
impl Env for ConcreteEnv {
    const MANUAL_RESIZE: bool = false;
    /// FIXME:
    /// We could also implement `Borrow<sanakirja::Env> for ConcreteEnv`
    /// instead of this reference.
    type RoTx<'db> = sanakirja::Txn<&'db sanakirja::Env>;
    type RwTx<'db> = sanakirja::MutTxn<&'db sanakirja::Env, ()>;

    #[cold]
    #[inline(never)] // called once.
    fn open<P: AsRef<Path>>(path: P) -> Result<Self, InitError> {
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

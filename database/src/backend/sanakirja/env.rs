//! Implementation of `trait Env` for `sanakirja`.

//---------------------------------------------------------------------------------------------------- Import
use std::path::Path;

use crate::{
    backend::sanakirja::types::SanakirjaDb, database::Database, env::Env, error::RuntimeError,
    table::Table,
};

//---------------------------------------------------------------------------------------------------- ConcreteEnv
/// A strongly typed, concrete database environment, backed by `sanakirja`.
pub struct ConcreteEnv(sanakirja::Env);

//---------------------------------------------------------------------------------------------------- Env Impl
impl Env for ConcreteEnv {
    /// TODO
    ///
    /// We could also implement `Borrow<sanakirja::Env> for ConcreteEnv`
    /// instead of this reference.
    type RoTx<'db> = sanakirja::Txn<&'db sanakirja::Env>;

    /// TODO
    type RwTx<'db> = sanakirja::MutTxn<&'db sanakirja::Env, ()>;

    //------------------------------------------------ Required
    #[cold]
    #[inline(never)] // called once.
    /// TODO
    /// # Errors
    /// TODO
    fn open<P: AsRef<Path>>(path: P) -> Result<Self, RuntimeError> {
        todo!()
    }

    /// TODO
    /// # Errors
    /// TODO
    fn sync(&self) -> Result<(), RuntimeError> {
        todo!()
    }

    #[inline]
    /// TODO
    /// # Errors
    /// TODO
    fn ro_tx(&self) -> Result<Self::RoTx<'_>, RuntimeError> {
        todo!()
    }

    #[inline]
    /// TODO
    /// # Errors
    /// TODO
    fn rw_tx(&self) -> Result<Self::RwTx<'_>, RuntimeError> {
        todo!()
    }

    #[cold]
    #[inline(never)] // called infrequently?.
    /// TODO
    /// # Errors
    /// TODO
    fn create_tables_if_needed<T: Table>(
        &self,
        tx_rw: &mut Self::RwTx<'_>,
    ) -> Result<(), RuntimeError> {
        todo!()
    }

    #[inline]
    /// TODO
    /// # Errors
    /// TODO
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

//! Concrete transaction types.
//!
//! These transactions are a combination of typical
//! "transaction" objects alongside an actual `K/V` table.
//!
//! This is done so callers don't need to
//! juggle around tables/transactions, they just:
//!
//! 1. Get a K/V table from the `Database` (1 single database)
//! 2. Do whatever they need to do (`get()`, `put()`, etc)

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    env::Env,
    error::{InitError, RuntimeError},
    table::Table,
    transaction::{RoTx, RwTx},
};

use std::path::Path;

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- RoTx
/// TODO
pub(super) struct ConcreteRoTx<'db, K, V> {
    /// TODO
    tx: heed::RoTxn<'db>,
    /// TODO
    db: &'db heed::Database<K, V>,
}

impl<K, V> RoTx<'_, K, V> for ConcreteRoTx<'_, K, V> {
    fn commit(self) -> Result<(), RuntimeError> {
        todo!()
    }

    fn get(&self, key: &K) -> Result<Option<V>, RuntimeError> {
        todo!()
    }

    #[allow(refining_impl_trait)] // TODO: add back `impl Iterator`
    fn get_range(&self, key: &K, amount: usize) -> Result<std::vec::Drain<'_, V>, RuntimeError> {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- RwTx
/// TODO
pub(super) struct ConcreteRwTx<'db, K, V> {
    /// TODO
    tx: heed::RwTxn<'db>,
    /// TODO
    db: &'db heed::Database<K, V>,
}

impl<K, V> RwTx<'_, K, V> for ConcreteRwTx<'_, K, V> {
    /// TODO
    /// # Errors
    /// TODO
    fn put(&mut self, key: &K, value: &V) -> Result<(), RuntimeError> {
        todo!()
    }

    /// TODO
    /// # Errors
    /// TODO
    fn clear(&mut self) -> Result<(), RuntimeError> {
        todo!()
    }

    /// TODO
    /// # Errors
    /// TODO
    fn delete(&mut self, key: &K) -> Result<bool, RuntimeError> {
        todo!()
    }

    /// TODO
    /// # Errors
    /// TODO
    fn commit(self) -> Result<(), RuntimeError> {
        todo!()
    }

    /// TODO
    fn abort(self) {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

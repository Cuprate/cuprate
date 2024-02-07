//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    database::Database,
    error::{InitError, RuntimeError},
    table::Table,
    transaction::{RoTx, RwTx},
};

use std::path::Path;

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- RoTx
/// TODO
pub struct ConcreteRoTx<'db, K, V> {
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
pub struct ConcreteRwTx<'db, K, V> {
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

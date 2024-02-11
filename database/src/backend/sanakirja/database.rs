//! Implementation of `trait Database` for `sanakirja`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{database::Database, error::RuntimeError, table::Table};

//---------------------------------------------------------------------------------------------------- Database Impls
impl<T: Table> Database<T> for sanakirja::btree::Db<T::Key, T::Value> {
    fn get(&self, key: &T::Key) -> Result<Option<T::Value>, RuntimeError> {
        todo!()
    }

    #[allow(refining_impl_trait)] // TODO: add back `impl Iterator`
    fn get_range(
        &self,
        key: &T::Key,
        amount: usize,
    ) -> Result<std::vec::Drain<'_, T::Value>, RuntimeError> {
        todo!()
    }

    fn put(&mut self, key: &T::Key, value: &T::Value) -> Result<(), RuntimeError> {
        todo!()
    }

    fn clear(&mut self) -> Result<(), RuntimeError> {
        todo!()
    }

    fn delete(&mut self, key: &T::Key) -> Result<bool, RuntimeError> {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

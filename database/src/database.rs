//! Abstracted database; `trait Database`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    error::RuntimeError,
    table::Table,
    transaction::{RoTx, RwTx},
};

//---------------------------------------------------------------------------------------------------- Database
/// Database (key-value store) abstraction.
///
/// TODO
pub trait Database<T: Table> {
    //------------------------------------------------ Types
    /// TODO
    type RoTx<'db>: RoTx<'db>;

    /// TODO
    type RwTx<'db>: RwTx<'db>;

    //-------------------------------------------------------- Read
    /// TODO
    /// # Errors
    /// TODO
    fn get(&self, ro_tx: &Self::RoTx<'_>, key: &T::Key) -> Result<Option<T::Value>, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn get_range(
        &self,
        ro_tx: &Self::RoTx<'_>,
        key: &T::Key,
        amount: usize,
    ) -> Result<impl Iterator<Item = T::Value>, RuntimeError>;

    //-------------------------------------------------------- Write
    /// TODO
    /// # Errors
    /// TODO
    fn put(
        &mut self,
        rw_tx: &mut Self::RwTx<'_>,
        key: &T::Key,
        value: &T::Value,
    ) -> Result<(), RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn clear(&mut self, rw_tx: &mut Self::RwTx<'_>) -> Result<(), RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn delete(&mut self, rw_tx: &mut Self::RwTx<'_>, key: &T::Key) -> Result<bool, RuntimeError>;
}

//! Database transaction abstraction; `trait RoTx`, `trait RwTx`.

//---------------------------------------------------------------------------------------------------- Import
use crate::error::RuntimeError;

//---------------------------------------------------------------------------------------------------- RoTx
/// Read-only database transaction.
///
/// TODO
pub trait RoTx<'db> {
    /// TODO
    /// # Errors
    /// TODO
    fn commit(self) -> Result<(), RuntimeError>;
}

//---------------------------------------------------------------------------------------------------- RwTx
/// Read/write database transaction.
///
/// TODO
pub trait RwTx<'db> {
    /// TODO
    /// # Errors
    /// TODO
    fn commit(self) -> Result<(), RuntimeError>;

    /// TODO
    fn abort(self);
}

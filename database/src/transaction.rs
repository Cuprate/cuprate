//! Database transaction abstraction; `trait TxRo`, `trait TxRw`.

//---------------------------------------------------------------------------------------------------- Import
use crate::error::RuntimeError;

//---------------------------------------------------------------------------------------------------- TxRo
/// Read-only database transaction.
///
/// TODO
pub trait TxRo<'db> {
    /// TODO
    /// # Errors
    /// TODO
    fn commit(self) -> Result<(), RuntimeError>;
}

//---------------------------------------------------------------------------------------------------- TxRw
/// Read/write database transaction.
///
/// TODO
pub trait TxRw<'db> {
    /// TODO
    /// # Errors
    /// TODO
    fn commit(self) -> Result<(), RuntimeError>;

    /// TODO
    fn abort(self);
}

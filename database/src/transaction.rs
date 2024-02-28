//! Database transaction abstraction; `trait TxRo`, `trait TxRw`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{config::SyncMode, env::Env, error::RuntimeError};

//---------------------------------------------------------------------------------------------------- TxRo
/// Read-only database transaction.
///
/// TODO
pub trait TxRo<'env> {
    /// TODO
    /// # Errors
    /// TODO
    fn commit(self) -> Result<(), RuntimeError>;
}

//---------------------------------------------------------------------------------------------------- TxRw
/// Read/write database transaction.
///
/// TODO
pub trait TxRw<'env> {
    /// TODO
    /// # Errors
    /// TODO
    fn commit(self) -> Result<(), RuntimeError>;

    /// TODO
    fn abort(self);
}

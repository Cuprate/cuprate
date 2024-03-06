//! Database transaction abstraction; `trait TxRo`, `trait TxRw`.

//---------------------------------------------------------------------------------------------------- Import
use crate::{config::SyncMode, env::Env, error::RuntimeError};

//---------------------------------------------------------------------------------------------------- TxCreator
/// Database transaction creator.
///
/// TODO
pub trait TxCreator<'env, Ro, Rw>
where
    Ro: TxRo<'env> + 'env,
    Rw: TxRw<'env> + 'env,
{
    /// TODO
    /// # Errors
    /// TODO
    fn tx_ro(&'env self) -> Result<Ro, RuntimeError>;

    /// TODO
    /// # Errors
    /// TODO
    fn tx_rw(&'env self) -> Result<Rw, RuntimeError>;
}

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

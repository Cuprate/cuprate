//! Database transaction abstraction; `trait TxRo`, `trait TxRw`.

//---------------------------------------------------------------------------------------------------- Import
use crate::error::RuntimeError;

//---------------------------------------------------------------------------------------------------- TxRo
/// Read-only database transaction.
///
/// Returned from [`EnvInner::tx_ro`](crate::EnvInner::tx_ro).
///
/// # TODO
/// I don't think we need this, we can just drop the `tx_ro`?
/// <https://docs.rs/heed/0.20.0-alpha.9/heed/struct.RoTxn.html#method.commit>
pub trait TxRo<'env> {
    /// Commit the read-only transaction.
    ///
    /// # Errors
    /// This operation is infallible (will always return `Ok(())`) with the `redb` backend.
    fn commit(self) -> Result<(), RuntimeError>;
}

//---------------------------------------------------------------------------------------------------- TxRw
/// Read/write database transaction.
///
/// Returned from [`EnvInner::tx_rw`](crate::EnvInner::tx_rw).
pub trait TxRw<'env> {
    /// Commit the read/write transaction.
    ///
    /// Note that this doesn't necessarily sync the database caches to disk.
    ///
    /// # Errors
    /// This operation is infallible (will always return `Ok(())`) with the `redb` backend.
    ///
    /// Else, this will only return:
    /// - [`RuntimeError::ResizeNeeded`] (if `Env::MANUAL_RESIZE == true`)
    /// - [`RuntimeError::Io`]
    fn commit(self) -> Result<(), RuntimeError>;

    /// Abort the transaction, erasing any writes that have occurred.
    ///
    /// # Errors
    /// This operation is infallible (will always return `Ok(())`) with the `heed` backend.
    ///
    /// Else, this will only return:
    /// - [`RuntimeError::ResizeNeeded`] (if `Env::MANUAL_RESIZE == true`)
    /// - [`RuntimeError::Io`]
    fn abort(self) -> Result<(), RuntimeError>;
}

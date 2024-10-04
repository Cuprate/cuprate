//! Database transaction abstraction; `trait TxRo`, `trait TxRw`.

//---------------------------------------------------------------------------------------------------- Import
use crate::error::RuntimeError;

//---------------------------------------------------------------------------------------------------- TxRo
/// Read-only database transaction.
///
/// Returned from [`EnvInner::tx_ro`](crate::EnvInner::tx_ro).
///
/// # Commit
/// It's recommended but may not be necessary to call [`TxRo::commit`] in certain cases:
/// - <https://docs.rs/heed/0.20.0-alpha.9/heed/struct.RoTxn.html#method.commit>
pub trait TxRo<'tx> {
    /// Commit the read-only transaction.
    ///
    /// # Errors
    /// This operation will always return `Ok(())` with the `redb` backend.
    fn commit(self) -> Result<(), RuntimeError>;
}

//---------------------------------------------------------------------------------------------------- TxRw
/// Read/write database transaction.
///
/// Returned from [`EnvInner::tx_rw`](crate::EnvInner::tx_rw).
pub trait TxRw<'tx> {
    /// Commit the read/write transaction.
    ///
    /// Note that this doesn't necessarily sync the database caches to disk.
    ///
    /// # Errors
    /// This operation will always return `Ok(())` with the `redb` backend.
    ///
    /// If `Env::MANUAL_RESIZE == true`,
    /// [`RuntimeError::ResizeNeeded`] may be returned.
    fn commit(self) -> Result<(), RuntimeError>;

    /// Abort the transaction, erasing any writes that have occurred.
    ///
    /// # Errors
    /// This operation will always return `Ok(())` with the `heed` backend.
    fn abort(self) -> Result<(), RuntimeError>;
}

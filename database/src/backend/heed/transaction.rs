//! Implementation of `trait TxRo/TxRw` for `heed`.

//---------------------------------------------------------------------------------------------------- Import
use std::sync::{RwLockReadGuard, RwLockWriteGuard};

use crate::{
    error::RuntimeError,
    transaction::{TxRo, TxRw},
};

//---------------------------------------------------------------------------------------------------- Transaction wrappers
// Q. Why does `HeedTxR{o,w}` exist?
// A. These wrapper types combine `heed`'s transaction
// types with the `Read/WriteGuard` returned by our
// `RwLock` wrapping the `heed::Env`. This is required
// otherwise `self.env.read().unwrap().read_txn()` returns
// a reference to the guard, which isn't allowed.
// Transactions themselves must hold onto the guard.

/// A read-only transaction.
pub(super) struct HeedTxRo<'env> {
    /// Read guard to the database environment.
    _guard: RwLockReadGuard<'env, heed::Env>,
    /// An active read-only transaction.
    tx_ro: heed::RoTxn<'env>,
}

/// A read/write transaction.
pub(super) struct HeedTxRw<'env> {
    /// Read guard to the database environment.
    _guard: RwLockReadGuard<'env, heed::Env>,
    /// An active read/write transaction.
    tx_rw: heed::RwTxn<'env>,
}

//---------------------------------------------------------------------------------------------------- TxRo
impl TxRo<'_> for HeedTxRo<'_> {
    fn commit(self) -> Result<(), RuntimeError> {
        self.tx_ro.commit().map_err(Into::into)
    }
}

//---------------------------------------------------------------------------------------------------- TxRw
impl TxRo<'_> for HeedTxRw<'_> {
    fn commit(self) -> Result<(), RuntimeError> {
        self.tx_rw.commit().map_err(Into::into)
    }
}

impl TxRw<'_> for HeedTxRw<'_> {
    fn commit(self) -> Result<(), RuntimeError> {
        self.tx_rw.commit().map_err(Into::into)
    }

    fn abort(self) {
        self.tx_rw.abort();
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

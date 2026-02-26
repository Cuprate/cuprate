//! Abstracted Monero tx-pool database operations.
//!
//! # `impl Table`
//!
//! As such, the responsibility of
//! transactions, tables, etc, are on the caller.
//!
//! Notably, this means that these functions are as lean
//! as possible, so calling them in a loop should be okay.
//!
//! # Atomicity
//! As transactions are handled by the _caller_ of these functions,
//! it is up to the caller to decide what happens if one them return
//! an error.
//!
//! For example, if [`add_transaction`] is called and returns an [`Err`],
//! `abort`ing the transaction that opened the input `TableMut` would reverse all tables
//! mutated by [`add_transaction`] up until the error, leaving it in the state it was in before
//! [`add_transaction`] was called.
//!

mod key_images;
mod tx_read;
mod tx_write;

use crate::error::TxPoolError;
pub use tx_read::{get_transaction_verification_data, in_stem_pool};
pub use tx_write::{add_transaction, remove_transaction};

/// An error that can occur on some tx-write ops.
#[derive(thiserror::Error, Debug)]
pub enum TxPoolWriteError {
    /// The transaction could not be added as it double spends another tx in the pool.
    ///
    /// The inner value is the hash of the transaction that was double spent.
    #[error("Transaction doubles spent transaction already in the pool ({}).", hex::encode(.0))]
    DoubleSpend(crate::types::TransactionHash),
    #[error("{0}")]
    TxPool(#[from] TxPoolError),
}

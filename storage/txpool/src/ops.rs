//! Abstracted Monero tx-pool database operations.
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

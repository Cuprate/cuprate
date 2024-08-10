mod key_images;
mod tx_read;
mod tx_write;

pub use tx_read::get_transaction_verification_data;
pub use tx_write::{add_transaction, remove_transaction};

#[derive(thiserror::Error, Debug)]
pub enum TxPoolWriteError {
    /// The transaction could not be added as it double spends another tx in the pool.
    ///
    /// The inner value is the hash of the transaction that was double spent.
    #[error("Transaction doubles spent transaction already in the pool ({}).", hex::encode(.0))]
    DoubleSpend(crate::types::TransactionHash),
    /// A database error.
    #[error("Database error: {0}")]
    Database(#[from] cuprate_database::RuntimeError),
}
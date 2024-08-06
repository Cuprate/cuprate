pub mod config;
mod free;
mod ops;
pub mod service;
mod tables;
mod types;

pub use config::Config;
pub use free::open;

//re-exports
pub use cuprate_database;

#[derive(thiserror::Error, Debug)]
pub enum TxPoolWriteError {
    /// The transaction could not be added as it double spends another tx in the pool.
    ///
    /// The inner value is the hash of the transaction that was double spent.
    #[error("Transaction doubles spent transaction already in the pool ({}).", hex::encode(.0))]
    DoubleSpend(types::TransactionHash),
    /// A database error.
    #[error("Database error: {0}")]
    Database(#[from] cuprate_database::RuntimeError),
}

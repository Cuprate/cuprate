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
//! # Example
//! Simple usage of `ops`.
//!
//! ```rust
//! use hex_literal::hex;
//!
//! use cuprate_test_utils::data::TX_V1_SIG2;
//! use cuprate_txpool::{
//!     cuprate_database::{
//!         ConcreteEnv,
//!         Env, EnvInner,
//!         DatabaseRo, DatabaseRw, TxRo, TxRw,
//!     },
//!     config::ConfigBuilder,
//!     tables::{Tables, TablesMut, OpenTables},
//!     ops::{add_transaction, get_transaction_verification_data},
//! };
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a configuration for the database environment.
//! let tmp_dir = tempfile::tempdir()?;
//! let db_dir = tmp_dir.path().to_owned();
//! let config = ConfigBuilder::new()
//!     .data_directory(db_dir.into())
//!     .build();
//!
//! // Initialize the database environment.
//! let env = cuprate_txpool::open(&config)?;
//!
//! // Open up a transaction + tables for writing.
//! let env_inner = env.env_inner();
//! let tx_rw = env_inner.tx_rw()?;
//! let mut tables = env_inner.open_tables_mut(&tx_rw)?;
//!
//! // Write a tx to the database.
//! let mut tx = TX_V1_SIG2.clone();
//! let tx_hash = tx.tx_hash;
//! add_transaction(&tx.try_into().unwrap(), true, &mut tables)?;
//!
//! // Commit the data written.
//! drop(tables);
//! TxRw::commit(tx_rw)?;
//!
//! // Read the data, assert it is correct.
//! let tx_rw = env_inner.tx_rw()?;
//! let mut tables = env_inner.open_tables_mut(&tx_rw)?;
//! let tx = get_transaction_verification_data(&tx_hash, &mut tables)?;
//!
//! assert_eq!(tx.tx_hash, tx_hash);
//! assert_eq!(tx.tx, TX_V1_SIG2.tx);
//! # Ok(()) }
//! ```

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

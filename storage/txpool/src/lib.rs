#![doc = include_str!("../README.md")]
#![allow(
    // See `cuprate-database` for reasoning.
    clippy::significant_drop_tightening
)]

// Used in docs: <https://github.com/Cuprate/cuprate/pull/170#discussion_r1823644357>.
use tower as _;

pub mod config;
mod free;
pub mod ops;
pub mod service;
pub mod tables;
mod tx;
pub mod types;

pub use config::Config;
pub use free::{open, transaction_blob_hash, DATABASE_VERSION, VERSION_KEY};
pub use tx::TxEntry;

//re-exports
pub use cuprate_database;

#[cfg(test)]
mod test {
    use cuprate_test_utils as _;
    use hex_literal as _;
    use tempfile as _;
    use tokio as _;
}

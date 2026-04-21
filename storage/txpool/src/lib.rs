#![doc = include_str!("../README.md")]
#![allow(
    // See `cuprate-database` for reasoning.
    clippy::significant_drop_tightening
)]

// Used in docs: <https://github.com/Cuprate/cuprate/pull/170#discussion_r1823644357>.
use tower as _;

mod error;
mod free;
pub mod ops;
pub mod service;
mod tx;
mod txpool;
pub mod types;

pub use error::TxPoolError;
pub use free::transaction_blob_hash;
pub use tx::TxEntry;

#[cfg(test)]
mod test {
    use cuprate_test_utils as _;
    use hex_literal as _;
    use tempfile as _;
}

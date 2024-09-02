#![doc = include_str!("../README.md")]
#![allow(
    // See `cuprate-database` for reasoning.
    clippy::significant_drop_tightening
)]

pub mod config;
mod free;
pub mod ops;
#[cfg(feature = "service")]
pub mod service;
pub mod tables;
pub mod types;

pub use config::Config;
pub use free::open;

//re-exports
pub use cuprate_database;

// TODO: remove when used.
use tower as _;
#[cfg(test)]
mod test {
    use cuprate_test_utils as _;
    use hex_literal as _;
    use tempfile as _;
    use tokio as _;
}

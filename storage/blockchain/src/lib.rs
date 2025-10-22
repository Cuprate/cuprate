#![doc = include_str!("../README.md")]
#![allow(
    // See `cuprate-database` for reasoning.
    clippy::significant_drop_tightening
)]
extern crate core;

// Only allow building 64-bit targets.
//
// This allows us to assume 64-bit
// invariants in code, e.g. `usize as u64`.
//
// # Safety
// As of 0d67bfb1bcc431e90c82d577bf36dd1182c807e2 (2024-04-12)
// there are invariants relying on 64-bit pointer sizes.
#[cfg(not(target_pointer_width = "64"))]
compile_error!("Cuprate is only compatible with 64-bit CPUs");

//---------------------------------------------------------------------------------------------------- Public API
// Import private modules, export public types.
//
// Documentation for each module is located in the respective file.

mod constants;
mod database;
mod free;

pub use constants::DATABASE_VERSION;
pub use cuprate_database;
pub use database::{BlockchainDatabase, BlockchainDatabaseService};
pub use free::open;

pub mod config;
pub mod ops;
pub mod service;
pub mod tables;
pub mod types;

//---------------------------------------------------------------------------------------------------- Private
#[cfg(test)]
pub(crate) mod tests;

pub(crate) mod unsafe_sendable;

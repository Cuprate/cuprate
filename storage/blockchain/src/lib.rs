#![doc = include_str!("../README.md")]
#![allow(
    // See `cuprate-database` for reasoning.
    clippy::significant_drop_tightening
)]

//---------------------------------------------------------------------------------------------------- Public API
// Import private modules, export public types.
//
// Documentation for each module is located in the respective file.

mod constants;
mod free;

pub use constants::DATABASE_VERSION;
pub use cuprate_database;
pub use free::open;

pub mod config;
pub mod ops;
pub mod tables;
pub mod types;

//---------------------------------------------------------------------------------------------------- Feature-gated
#[cfg(feature = "service")]
pub mod service;

//---------------------------------------------------------------------------------------------------- Private
#[cfg(test)]
pub(crate) mod tests;

#[cfg(feature = "service")] // only needed in `service` for now
pub(crate) mod unsafe_sendable;

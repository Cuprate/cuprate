//! Abstracted Monero database operations.
//!
//! This module contains many free functions that use the
//! traits in this crate to generically call Monero-related
//! database operations.
//!
//! # TODO
//! TODO: These functions should pretty much map 1-1 to the `Request` enum.
//!
//! TODO: These are function names from `old_database/` for now.
//! The actual underlying functions (e.g `get()`) aren't implemented.
//!
//! TODO: All of these functions need to take in generic
//! database trait parameters (and their actual inputs).

pub mod alt_block;
pub mod block;
pub mod output;
pub mod property;
pub mod spent_key;
pub mod tx;

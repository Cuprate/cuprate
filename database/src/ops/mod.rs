//! Abstracted Monero database operations.
//!
//! This module contains many free functions that use the
//! traits in this crate to generically call Monero-related
//! database operations.
//!
//! # `_bulk()`
//! TODO: explain bulk functions.
//!
//! # Atomicity
//! TODO: explain atomic behavior of `ops/` functions.
//!
//! # Generic Inputs
//! TODO: explain `<'env, Ro, Rw, EnvInner>` - show examples of setups and fn calls.
//!
//! # TODO
//! TODO: These functions should pretty much map 1-1 to the `Request` enum.

pub mod alt_block;
pub mod block;
pub mod blockchain;
pub mod output;
pub mod property;
pub mod spent_key;
pub mod tx;

mod macros;

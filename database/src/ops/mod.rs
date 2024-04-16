//! Abstracted Monero database operations.
//!
//! This module contains many free functions that use the
//! traits in this crate to generically call Monero-related
//! database operations.
//!
//! # `impl Table`
//! TODO: explain how functions take open tables directly, why, and how to use them.
//! Show examples of setups and fn calls.
//!
//! # Atomicity
//! TODO: explain atomic behavior of `ops/` functions.
//!
//! # TODO
//! TODO: These functions should pretty much map 1-1 to the `Request` enum.

// pub mod alt_block; // TODO: is this needed?
pub mod block;
pub mod blockchain;
pub mod key_image;
pub mod output;
pub mod property;
pub mod tx;

mod macros;

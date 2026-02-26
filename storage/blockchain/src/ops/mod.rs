//! Abstracted Monero database operations.
//!
//! This module contains many free functions that use the
//! traits in this crate to generically call Monero-related
//! database operations.
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
//! # Sub-functions
//! The main functions within this module are mostly within the [`block`] module.
//!
//! Practically speaking, you should only be using 2 functions for mutation:
//!
//! The `block` functions are "parent" functions, calling other
//! sub-functions such as [`add_output()`](output::add_output).
//!
//! `add_output()` itself only modifies output-related tables, while the `block` "parent"
//! functions (like `add_block` and `pop_block`) modify all tables required.
//!
//! `add_block()` makes sure all data related to the input is mutated, while
//! this sub-function _do not_, it specifically mutates _particular_ tables.
//!
//! When calling this sub-functions, ensure that either:
//! 1. This effect (incomplete database mutation) is what is desired, or that...
//! 2. ...the other tables will also be mutated to a correct state
//!

pub mod alt_block;
pub mod block;
pub mod blockchain;
pub mod output;
pub mod property;
pub mod tx;

mod macros;

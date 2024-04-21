//! Abstracted Monero database operations.
//!
//! This module contains many free functions that use the
//! traits in this crate to generically call Monero-related
//! database operations.
//!
//! # `impl Table`
//! `ops/` functions take [`Tables`](crate::tables::Tables) and
//! [`TablesMut`](crate::tables::TablesMut) directly - these are
//! _already opened_ database tables.
//!
//! As such, the function puts the responsibility
//! of transactions, tables, etc on the caller.
//!
//! This does mean these functions are mostly as lean
//! as possible, so calling them in a loop should be okay.
//!
//! # Atomicity
//! As transactions are handled by the _caller_ of these functions,
//! it is up to the caller to decide what happens if one them return
//! an error.
//!
//! To maintain atomicity, transactions should be [`abort`](crate::transaction::TxRw::abort)ed
//! if one of the functions failed.
//!
//! For example, if [`add_block()`](block::add_block) is called and returns an [`Err`],
//! `abort`ing the transaction that opened the input `TableMut` would reverse all tables
//! mutated by `add_block()` up until the error, leaving it in the state it was in before
//! `add_block()` was called.
//!
//! # Sub-functions
//! The main functions within this module are mostly within the [`block`] module.
//!
//! Practically speaking, you should only be using 2 functions:
//! - [`add_block`](block::add_block)
//! - [`pop_block`](block::pop_block)
//!
//! The `block` functions are "parent" functions, calling other
//! sub-functions such as [`add_output()`](output::add_output). `add_output()`
//! itself only modifies output-related tables, while the `block` "parent" functions
//! (like `add_block` and `pop_block`) modify _everything_ that is required.

// pub mod alt_block; // TODO: is this needed?
pub mod block;
pub mod blockchain;
pub mod key_image;
pub mod output;
pub mod property;
pub mod tx;

mod macros;

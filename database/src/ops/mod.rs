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
//!
//! # Example
//! Simple usage of `ops`.
//!
//! ```rust
//! use hex_literal::hex;
//!
//! use cuprate_test_utils::data::block_v16_tx0;
//!
//! use cuprate_database::{
//!     ConcreteEnv,
//!     config::Config,
//!     Env, EnvInner,
//!     tables::{Tables, TablesMut},
//!     DatabaseRo, DatabaseRw, TxRo, TxRw,
//!     ops::block::{add_block, pop_block},
//! };
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a configuration for the database environment.
//! let db_dir = tempfile::tempdir()?;
//! let config = Config::new(Some(db_dir.path().to_path_buf()));
//!
//! // Initialize the database environment.
//! let env = ConcreteEnv::open(config)?;
//!
//! // Open up a transaction + tables for writing.
//! let env_inner = env.env_inner();
//! let tx_rw = env_inner.tx_rw()?;
//! let mut tables = env_inner.open_tables_mut(&tx_rw)?;
//!
//! // Write a block to the database.
//! let mut block = block_v16_tx0().clone();
//! # block.height = 0;
//! add_block(&block, &mut tables)?;
//!
//! // Commit the data written.
//! drop(tables);
//! TxRw::commit(tx_rw)?;
//!
//! // Read the data, assert it is correct.
//! let tx_rw = env_inner.tx_rw()?;
//! let mut tables = env_inner.open_tables_mut(&tx_rw)?;
//! let (height, hash, serai_block) = pop_block(&mut tables)?;
//!
//! assert_eq!(height, 0);
//! assert_eq!(serai_block, block.block);
//! assert_eq!(hash, hex!("43bd1f2b6556dcafa413d8372974af59e4e8f37dbf74dc6b2a9b7212d0577428"));
//! # Ok(()) }
//! ```

pub mod block;
pub mod blockchain;
pub mod key_image;
pub mod output;
pub mod property;
pub mod tx;

mod macros;

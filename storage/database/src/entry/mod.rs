//! `(key, value)` entry API for [`crate::DatabaseRw`].
//!
//! This module provides a [`std::collections::btree_map::Entry`]-like API for [`crate::DatabaseRw`].
//!
//! ## Example - modifying a value in place, or inserting it if it doesn't exist
//! ```rust
//! use cuprate_database::{
//!     ConcreteEnv,
//!     config::ConfigBuilder,
//!     Env, EnvInner,
//!     DatabaseRo, DatabaseRw, TxRo, TxRw, RuntimeError,
//! };
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let tmp_dir = tempfile::tempdir()?;
//! # let db_dir = tmp_dir.path().to_owned();
//! # let config = ConfigBuilder::new(db_dir.into()).build();
//! #
//! # let env = ConcreteEnv::open(config)?;
//! #
//! # struct Table;
//! # impl cuprate_database::Table for Table {
//! #     const NAME: &'static str = "table";
//! #     type Key = u8;
//! #     type Value = u64;
//! # }
//! #
//! # let env_inner = env.env_inner();
//! # let tx_rw = env_inner.tx_rw()?;
//! #
//! # env_inner.create_db::<Table>(&tx_rw)?;
//! # let mut table = env_inner.open_db_rw::<Table>(&tx_rw)?;
//! /// The key to use.
//! const KEY: u8 = u8::MAX;
//!
//! /// The update function applied if the value already exists.
//! fn f(value: &mut u64) {
//!     *value += 1;
//! }
//!
//! // No entry exists.
//! assert!(matches!(table.first(), Err(RuntimeError::KeyNotFound)));
//!
//! // Increment the value by `1` or insert a `0` if it doesn't exist.
//! table.entry(&KEY)?.and_update(f)?.or_insert(&0)?;
//! assert_eq!(table.first()?, (KEY, 0));
//!
//! // Again.
//! table.entry(&KEY)?.and_update(f)?.or_insert(&0)?;
//! assert_eq!(table.first()?, (KEY, 1));
//! # Ok(()) }
//! ```

#[expect(clippy::module_inception)]
mod entry;
mod occupied_entry;
mod vacant_entry;

pub use entry::Entry;
pub use occupied_entry::OccupiedEntry;
pub use vacant_entry::VacantEntry;

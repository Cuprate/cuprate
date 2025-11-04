#![doc = include_str!("../README.md")]
#![allow(
    // This lint is allowed because the following
    // code exists a lot in this crate:
    //
    // ```rust
    // let env_inner = env.env_inner();
    // let tx_rw = env_inner.tx_rw()?;
    // OpenTables::create_tables(&env_inner, &tx_rw)?;
    // ```
    //
    // Rust thinks `env_inner` can be dropped earlier
    // but it cannot, we need it for the lifetime of
    // the database transaction + tables.
    clippy::significant_drop_tightening,
unreachable_pub
)]
// Allow some lints in tests.
#![cfg_attr(
    test,
    allow(
        clippy::cognitive_complexity,
        clippy::needless_pass_by_value,
        clippy::cast_possible_truncation,
        clippy::too_many_lines
    )
)]

//---------------------------------------------------------------------------------------------------- Public API
// Import private modules, export public types.
//
// Documentation for each module is located in the respective file.

mod backend;
mod constants;
mod database;
mod env;
mod error;
mod key;
mod storable;
mod table;
mod tables;
mod transaction;

pub mod config;
pub mod resize;

pub use backend::ConcreteEnv;
pub use constants::{
    DATABASE_BACKEND, DATABASE_CORRUPT_MSG, DATABASE_DATA_FILENAME, DATABASE_LOCK_FILENAME,
};
pub use database::{DatabaseIter, DatabaseRo, DatabaseRw};
pub use env::{Env, EnvInner};
pub use error::{DbResult, InitError, RuntimeError};
pub use key::{Key, KeyCompare};
pub use storable::{Storable, StorableBytes, StorableStr, StorableVec};
pub use table::Table;
pub use transaction::{TxRo, TxRw};

//---------------------------------------------------------------------------------------------------- Private
#[cfg(test)]
pub(crate) mod tests;

// Used inside public facing macros.
#[doc(hidden)]
pub use paste;

//----------------------------------------------------------------------------------------------------
// HACK: needed to satisfy the `unused_crate_dependencies` lint.
cfg_if::cfg_if! {
    if #[cfg(feature = "redb")]  {
        use redb as _;
    } else {
        use heed as _;
    }
}

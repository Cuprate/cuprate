//! Cuprate's database abstraction.
//!
//! This documentation is mostly for practical usage of `cuprate_blockchain`.
//!
//! For a high-level overview,
//! see [`database/README.md`](https://github.com/Cuprate/cuprate/blob/main/database/README.md).
//!
//! # Purpose
//! This crate does 3 things:
//! 1. Abstracts various database backends with traits
//! 2. Implements various `Monero` related [operations](ops), [tables], and [types]
//! 3. Exposes a [`tower::Service`] backed by a thread-pool
//!
//! Each layer builds on-top of the previous.
//!
//! As a user of `cuprate_blockchain`, consider using the higher-level [`service`] module,
//! or at the very least the [`ops`] module instead of interacting with the database traits directly.
//!
//! With that said, many database traits and internals (like [`DatabaseRo::get`]) are exposed.
//!
//! # Terminology
//! To be more clear on some terms used in this crate:
//!
//! | Term             | Meaning                              |
//! |------------------|--------------------------------------|
//! | `Env`            | The 1 database environment, the "whole" thing
//! | `DatabaseR{o,w}` | A _actively open_ readable/writable `key/value` store
//! | `Table`          | Solely the metadata of a `Database` (the `key` and `value` types, and the name)
//! | `TxR{o,w}`       | A read/write transaction
//! | `Storable`       | A data that type can be stored in the database
//!
//! The dataflow is `Env` -> `Tx` -> `Database`
//!
//! Which reads as:
//! 1. You have a database `Environment`
//! 1. You open up a `Transaction`
//! 1. You open a particular `Table` from that `Environment`, getting a `Database`
//! 1. You can now read/write data from/to that `Database`
//!
//! # `ConcreteEnv`
//! This crate exposes [`ConcreteEnv`], which is a non-generic/non-dynamic,
//! concrete object representing a database [`Env`]ironment.
//!
//! The actual backend for this type is determined via feature flags.
//!
//! This object existing means `E: Env` doesn't need to be spread all through the codebase,
//! however, it also means some small invariants should be kept in mind.
//!
//! As `ConcreteEnv` is just a re-exposed type which has varying inner types,
//! it means some properties will change depending on the backend used.
//!
//! For example:
//! - [`std::mem::size_of::<ConcreteEnv>`]
//! - [`std::mem::align_of::<ConcreteEnv>`]
//!
//! Things like these functions are affected by the backend and inner data,
//! and should not be relied upon. This extends to any `struct/enum` that contains `ConcreteEnv`.
//!
//! `ConcreteEnv` invariants you can rely on:
//! - It implements [`Env`]
//! - Upon [`Drop::drop`], all database data will sync to disk
//!
//! Note that `ConcreteEnv` itself is not a clonable type,
//! it should be wrapped in [`std::sync::Arc`].
//!
//! <!-- SOMEDAY: replace `ConcreteEnv` with `fn Env::open() -> impl Env`/
//! and use `<E: Env>` everywhere it is stored instead. This would allow
//! generic-backed dynamic runtime selection of the database backend, i.e.
//! the user can select which database backend they use. -->
//!
//! # Feature flags
//! The `service` module requires the `service` feature to be enabled.
//! See the module for more documentation.
//!
//! Different database backends are enabled by the feature flags:
//! - `heed` (LMDB)
//! - `redb`
//!
//! The default is `heed`.
//!
//! `tracing` is always enabled and cannot be disabled via feature-flag.
//! <!-- FIXME: tracing should be behind a feature flag -->
//!
//! # Invariants when not using `service`
//! `cuprate_blockchain` can be used without the `service` feature enabled but
//! there are some things that must be kept in mind when doing so.
//!
//! Failing to uphold these invariants may cause panics.
//!
//! 1. `LMDB` requires the user to resize the memory map resizing (see [`RuntimeError::ResizeNeeded`]
//! 1. `LMDB` has a maximum reader transaction count, currently it is set to `128`
//! 1. `LMDB` has [maximum key/value byte size](http://www.lmdb.tech/doc/group__internal.html#gac929399f5d93cef85f874b9e9b1d09e0) which must not be exceeded
//!
//! # Examples
//! The below is an example of using `cuprate_blockchain`'s
//! lowest API, i.e. using the database directly.
//!
//! For examples of the higher-level APIs, see:
//! - [`ops`]
//! - [`service`]
//!
//! ```rust
//! use cuprate_blockchain::{
//!     ConcreteEnv,
//!     config::ConfigBuilder,
//!     Env, EnvInner,
//!     tables::{Tables, TablesMut},
//!     DatabaseRo, DatabaseRw, TxRo, TxRw,
//! };
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a configuration for the database environment.
//! let db_dir = tempfile::tempdir()?;
//! let config = ConfigBuilder::new()
//!     .db_directory(db_dir.path().to_path_buf())
//!     .build();
//!
//! // Initialize the database environment.
//! let env = ConcreteEnv::open(config)?;
//!
//! // Open up a transaction + tables for writing.
//! let env_inner = env.env_inner();
//! let tx_rw = env_inner.tx_rw()?;
//! let mut tables = env_inner.open_tables_mut(&tx_rw)?;
//!
//! // ⚠️ Write data to the tables directly.
//! // (not recommended, use `ops` or `service`).
//! const KEY_IMAGE: [u8; 32] = [88; 32];
//! tables.key_images_mut().put(&KEY_IMAGE, &())?;
//!
//! // Commit the data written.
//! drop(tables);
//! TxRw::commit(tx_rw)?;
//!
//! // Read the data, assert it is correct.
//! let tx_ro = env_inner.tx_ro()?;
//! let tables = env_inner.open_tables(&tx_ro)?;
//! let (key_image, _) = tables.key_images().first()?;
//! assert_eq!(key_image, KEY_IMAGE);
//! # Ok(()) }
//! ```

//---------------------------------------------------------------------------------------------------- Lints
// Forbid lints.
// Our code, and code generated (e.g macros) cannot overrule these.
#![forbid(
	// `unsafe` is allowed but it _must_ be
	// commented with `SAFETY: reason`.
	clippy::undocumented_unsafe_blocks,

	// Never.
	unused_unsafe,
	redundant_semicolons,
	unused_allocation,
	coherence_leak_check,
	while_true,
	clippy::missing_docs_in_private_items,

	// Maybe can be put into `#[deny]`.
	unconditional_recursion,
	for_loops_over_fallibles,
	unused_braces,
	unused_labels,
	keyword_idents,
	non_ascii_idents,
	variant_size_differences,
    single_use_lifetimes,

	// Probably can be put into `#[deny]`.
	future_incompatible,
	let_underscore,
	break_with_label_and_loop,
	duplicate_macro_attributes,
	exported_private_dependencies,
	large_assignments,
	overlapping_range_endpoints,
	semicolon_in_expressions_from_macros,
	noop_method_call,
	unreachable_pub,
)]
// Deny lints.
// Some of these are `#[allow]`'ed on a per-case basis.
#![deny(
    clippy::all,
    clippy::correctness,
    clippy::suspicious,
    clippy::style,
    clippy::complexity,
    clippy::perf,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
    unused_doc_comments,
    unused_mut,
    missing_docs,
    deprecated,
    unused_comparisons,
    nonstandard_style
)]
#![allow(
	// FIXME: this lint affects crates outside of
	// `database/` for some reason, allow for now.
	clippy::cargo_common_metadata,

	// FIXME: adding `#[must_use]` onto everything
	// might just be more annoying than useful...
	// although it is sometimes nice.
	clippy::must_use_candidate,

	// FIXME: good lint but too many false positives
	// with our `Env` + `RwLock` setup.
	clippy::significant_drop_tightening,

	// FIXME: good lint but is less clear in most cases.
	clippy::items_after_statements,

	clippy::module_name_repetitions,
	clippy::module_inception,
	clippy::redundant_pub_crate,
	clippy::option_if_let_else,
)]
// Allow some lints when running in debug mode.
#![cfg_attr(debug_assertions, allow(clippy::todo, clippy::multiple_crate_versions))]
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
pub use backend::ConcreteEnv;

pub mod config;

mod constants;
pub use constants::{
    DATABASE_BACKEND, DATABASE_CORRUPT_MSG, DATABASE_DATA_FILENAME, DATABASE_LOCK_FILENAME,
    DATABASE_VERSION,
};

mod database;
pub use database::{DatabaseIter, DatabaseRo, DatabaseRw};

mod env;
pub use env::{Env, EnvInner};

mod error;
pub use error::{InitError, RuntimeError};

pub mod resize;

mod key;
pub use key::Key;

mod storable;
pub use storable::{Storable, StorableBytes, StorableVec};

mod table;
pub use table::Table;

mod transaction;
pub use transaction::{TxRo, TxRw};

//---------------------------------------------------------------------------------------------------- Private
#[cfg(test)]
pub(crate) mod tests;

//! Database abstraction and utilities.
//!
//! This documentation is mostly for practical usage of `cuprate_database`.
//!
//! For a high-level overview,
//! see [`database/README.md`](https://github.com/Cuprate/cuprate/blob/main/database/README.md).
//!
//! # Purpose
//! This crate does 3 things:
//! 1. Abstracts various database backends with traits
//! 2. Implements various `Monero` related [functions](ops) & [tables] & [types]
//! 3. Exposes a [`tower::Service`] backed by a thread-pool
//!
//! # Terminology
//! To be more clear on some terms used in this crate:
//!
//! | Term          | Meaning                              |
//! |---------------|--------------------------------------|
//! | `Env`         | The 1 database environment, the "whole" thing
//! | `DatabaseRo`  | A read-only `key/value` store
//! | `DatabaseRw`  | A readable/writable `key/value` store
//! | `Table`       | Solely the metadata of a `Database` (the `key` and `value` types, and the name)
//! | `TxRo`        | Read only transaction
//! | `TxRw`        | Read/write transaction
//! | `Storable`    | A data that type can be stored in the database
//!
//! The dataflow is `Env` -> `Tx` -> `Database`
//!
//! Which reads as:
//! 1. You have a database `Environment`
//! 1. You open up a `Transaction`
//! 1. You get a particular `Database` from that `Environment`
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
//! TODO: we could also expose `ConcreteDatabase` if we're
//! going to be storing any databases in structs, to lessen
//! the generic `<D: Database>` pain.
//!
//! TODO: we could replace `ConcreteEnv` with `fn Env::open() -> impl Env`/
//! and use `<E: Env>` everywhere it is stored instead. This would allow
//! generic-backed dynamic runtime selection of the database backend, i.e.
//! the user can select which database backend they use.
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
//! # Invariants when not using `service`
//! `cuprate_database` can be used without the `service` feature enabled but
//! there are some things that must be kept in mind when doing so:
//!
//! TODO: make pretty. these will need to be updated
//! as things change and as more backends are added.
//!
//! 1. Memory map resizing (must resize as needed)
//! 1. Must not exceed `Config`'s maximum reader count
//! 1. Avoid many nested transactions
//! 1. `heed::MdbError::BadValSize`
//! 1. `heed::Error::InvalidDatabaseTyping`
//! 1. `heed::Error::BadOpenOptions`
//! 1. Encoding/decoding into `[u8]`
//!
//! # Example
//! Simple usage of this crate.
//!
//! ```rust
//! use cuprate_database::{
//!     config::Config,
//!     ConcreteEnv,
//!     Env, Key, TxRo, TxRw,
//!     service::{ReadRequest, WriteRequest, Response},
//! };
//!
//! // Create a configuration for the database environment.
//! let db_dir = tempfile::tempdir().unwrap();
//! let config = Config::new(Some(db_dir.path().to_path_buf()));
//!
//! // Initialize the database thread-pool.
//!
//! // TODO:
//! // 1. let (read_handle, write_handle) = cuprate_database::service::init(config).unwrap();
//! // 2. Send write/read requests
//! // 3. Use some other `Env` functions
//! // 4. Shutdown
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
	single_use_lifetimes,
	while_true,
	clippy::missing_docs_in_private_items,

	// Maybe can be put into `#[deny]`.
	unconditional_recursion,
	for_loops_over_fallibles,
	unused_braces,
	unused_doc_comments,
	unused_labels,
	keyword_idents,
	non_ascii_idents,
	variant_size_differences,
	unused_mut, // Annoying when debugging, maybe put in allow.

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
    missing_docs,
    deprecated,
    unused_comparisons,
    nonstandard_style
)]
#![allow(unreachable_code, unused_variables, dead_code, unused_imports)] // TODO: remove
#![allow(
	// FIXME: this lint affects crates outside of
	// `database/` for some reason, allow for now.
	clippy::cargo_common_metadata,

	// FIXME: adding `#[must_use]` onto everything
	// might just be more annoying than useful...
	// although it is sometimes nice.
	clippy::must_use_candidate,

	// TODO: should be removed after all `todo!()`'s are gone.
	clippy::diverging_sub_expression,

	clippy::module_name_repetitions,
	clippy::module_inception,
	clippy::redundant_pub_crate,
	clippy::option_if_let_else,
)]
// Allow some lints when running in debug mode.
#![cfg_attr(debug_assertions, allow(clippy::todo, clippy::multiple_crate_versions))]

// Only allow building 64-bit targets.
//
// This allows us to assume 64-bit
// invariants in code, e.g. `usize as u64`.
//
// # Safety
// As of 0d67bfb1bcc431e90c82d577bf36dd1182c807e2 (2024-04-12)
// there are invariants relying on 64-bit pointer sizes.
#[cfg(not(target_pointer_width = "64"))]
compile_error!("Cuprate is only compatible with 64-bit CPUs");

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
};

mod database;
pub use database::{DatabaseRo, DatabaseRw};

mod env;
pub use env::{Env, EnvInner};

mod error;
pub use error::{InitError, RuntimeError};

mod free;

pub mod resize;

mod key;
pub use key::Key;

mod macros;

mod storable;
pub use storable::Storable;

pub mod ops;

mod table;
pub use table::Table;

pub mod tables;

pub mod types;

mod transaction;
pub use transaction::{TxRo, TxRw};

mod to_owned_debug;
pub use to_owned_debug::ToOwnedDebug;

mod value_guard;
pub use value_guard::ValueGuard;

//---------------------------------------------------------------------------------------------------- Feature-gated
#[cfg(feature = "service")]
pub mod service;

//---------------------------------------------------------------------------------------------------- Private

//! JSON-RPC 2.0 types and (de)serialization.
//!
//! ## What
//! This crate implements the [JSON-RPC 2.0 specification](https://www.jsonrpc.org/specification)
//! for usage in [Cuprate](https://github.com/Cuprate/cuprate).
//!
//! It contains slight modifications catered towards Cuprate and isn't
//! necessarily a general purpose implementation of the specification
//! (see below).
//!
//! ## Response changes
//! [JSON-RPC 2.0's `Response` object](https://www.jsonrpc.org/specification#response_object) usually contains these 2 fields:
//! - `method`
//! - `params`
//!
//! This crate replaces those two with a `body` field that is `#[serde(flatten)]`ed,
//! and assumes the type within that `body` field is tagged properly, for example:
//!
//! ```rust
//! # use pretty_assertions::assert_eq;
//! use serde::{Deserialize, Serialize};
//! use json_rpc::{Id, Request};
//!
//! // Parameter type.
//! #[derive(Deserialize, Serialize)]
//! struct GetBlock {
//!     height: u64,
//! }
//!
//! // Method enum containing all enums.
//! // All methods are tagged as `method`
//! // and their inner parameter types are
//! // tagged with `params` (in snake case).
//! #[derive(Deserialize, Serialize)]
//! #[serde(tag = "method", content = "params")] // INVARIANT: these tags are needed
//! #[serde(rename_all = "snake_case")]          // for proper (de)serialization.
//! enum Methods {
//!     GetBlock(GetBlock),
//!     /* other methods */
//! }
//!
//! // Create the request object.
//! let request = Request::new_with_id(
//!     Id::Str("hello".into()),
//!     Methods::GetBlock(GetBlock { height: 123 }),
//! );
//!
//! // Serializing properly shows the `method/params` fields
//! // even though `Request` doesn't contain those fields.
//! let json = serde_json::to_string_pretty(&request).unwrap();
//! let expected_json =
//! r#"{
//!   "jsonrpc": "2.0",
//!   "id": "hello",
//!   "method": "get_block",
//!   "params": {
//!     "height": 123
//!   }
//! }"#;
//! assert_eq!(json, expected_json);
//! ```
//!
//! This is how the method/param types are done in Cuprate.
//!
//! For reasoning, see: <https://github.com/Cuprate/cuprate/pull/146#issuecomment-2145734838>.
//!
//! ## Batching
//! This crate does not have any types for [JSON-RPC 2.0 batching](https://www.jsonrpc.org/specification#batch).
//!
//! This is because `monerod` does not support this,
//! as such, neither does Cuprate.
//!
//! TODO: citation needed on `monerod` not supporting batching.

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
    clippy::missing_docs_in_private_items,
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

//---------------------------------------------------------------------------------------------------- Mod/Use
pub mod error;

mod key;

mod id;
pub use id::*;

mod version;
pub use version::*;

mod request;
pub use request::*;

mod response;
pub use response::*;

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod tests;

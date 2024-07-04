#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
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
    nonstandard_style,
    unreachable_pub
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

	// TODO
	rustdoc::bare_urls,

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
// TODO: remove me after finishing impl
#![allow(dead_code)]

//---------------------------------------------------------------------------------------------------- Use
mod binary_string;
mod constants;
mod defaults;
mod free;
mod macros;
mod status;

pub use binary_string::BinaryString;
pub use constants::{
    CORE_RPC_STATUS_BUSY, CORE_RPC_STATUS_NOT_MINING, CORE_RPC_STATUS_OK,
    CORE_RPC_STATUS_PAYMENT_REQUIRED, CORE_RPC_STATUS_UNKNOWN, CORE_RPC_VERSION,
    CORE_RPC_VERSION_MAJOR, CORE_RPC_VERSION_MINOR,
};
pub use status::Status;

pub mod base;
#[cfg(feature = "bin")]
#[cfg_attr(docsrs, doc(cfg(feature = "bin")))]
pub mod bin;
#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
pub mod json;
#[cfg(feature = "other")]
#[cfg_attr(docsrs, doc(cfg(feature = "other")))]
pub mod other;

mod misc;
pub use misc::{BlockHeader, ConnectionInfo};

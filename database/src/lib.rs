//! TODO

//---------------------------------------------------------------------------------------------------- Lints
// Forbid lints.
// Our code, and code generated (e.g macros) cannot overrule these.
#![forbid(
	// Never.
	unsafe_code,
	unused_unsafe,
	redundant_semicolons,
	unused_allocation,
	coherence_leak_check,
	single_use_lifetimes,
	while_true,
	missing_docs,
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

	// FIXME:
	// If #[deny(clippy::restriction)] is used, it
	// enables a whole bunch of very subjective lints.
	// The below disables most of the ones that are
	// a bit too unwieldly.
	//
	// Figure out if if `clippy::restriction` should be
	// used (it enables a bunch of good lints but has
	// many false postives).

	// clippy::single_char_lifetime_names,
	// clippy::implicit_return,
	// clippy::std_instead_of_alloc,
	// clippy::std_instead_of_core,
	// clippy::unwrap_used,
	// clippy::min_ident_chars,
	// clippy::absolute_paths,
	// clippy::missing_inline_in_public_items,
	// clippy::shadow_reuse,
	// clippy::shadow_unrelated,
	// clippy::missing_trait_methods,
	// clippy::pub_use,
	// clippy::pub_with_shorthand,
	// clippy::blanket_clippy_restriction_lints,
	// clippy::exhaustive_structs,
	// clippy::exhaustive_enums,
	// clippy::unsafe_derive_deserialize,
	// clippy::multiple_inherent_impl,
	// clippy::unreadable_literal,
	// clippy::indexing_slicing,
	// clippy::float_arithmetic,
	// clippy::cast_possible_truncation,
	// clippy::as_conversions,
	// clippy::cast_precision_loss,
	// clippy::cast_sign_loss,
	// clippy::missing_asserts_for_indexing,
	// clippy::default_numeric_fallback,
	// clippy::module_inception,
	// clippy::mod_module_files,
	// clippy::multiple_unsafe_ops_per_block,
	// clippy::too_many_lines,
	// clippy::missing_assert_message,
	// clippy::len_zero,
	// clippy::separated_literal_suffix,
	// clippy::single_call_fn,
	// clippy::unreachable,
	// clippy::many_single_char_names,
	// clippy::redundant_pub_crate,
	// clippy::decimal_literal_representation,
	// clippy::option_if_let_else,
	// clippy::lossy_float_literal,
	// clippy::modulo_arithmetic,
	// clippy::print_stdout,
	// clippy::module_name_repetitions,
	// clippy::no_effect,
	// clippy::semicolon_outside_block,
	// clippy::panic,
	// clippy::question_mark_used,
	// clippy::expect_used,
	// clippy::integer_division,
	// clippy::type_complexity,
	// clippy::pattern_type_mismatch,
	// clippy::arithmetic_side_effects,
	// clippy::default_trait_access,
	// clippy::similar_names,
	// clippy::needless_pass_by_value,
	// clippy::inline_always,
	// clippy::if_then_some_else_none,
	// clippy::arithmetic_side_effects,
	// clippy::float_cmp,
	// clippy::items_after_statements,
	// clippy::use_debug,
	// clippy::mem_forget,
	// clippy::else_if_without_else,
	// clippy::str_to_string,
	// clippy::branches_sharing_code,
	// clippy::impl_trait_in_params,
	// clippy::struct_excessive_bools,
	// clippy::exit,
	// // This lint is actually good but
	// // it sometimes hits false positive.
	// clippy::self_named_module_files

	clippy::module_name_repetitions,
)]
// Allow some lints when running in debug mode.
#![cfg_attr(
    debug_assertions,
    allow(clippy::todo, clippy::multiple_crate_versions,)
)]

// Only allow building 64-bit targets.
//
// This allows us to assume 64-bit
// invariants in code, e.g. `usize as u64`.
#[cfg(not(target_pointer_width = "64"))]
compile_error!("Cuprate is only compatible with 64-bit CPUs");

//---------------------------------------------------------------------------------------------------- Public API
// Import private modules, export public types.
//
// Documentation for each module is
// located in the respective file.

mod backend;
pub use backend::{ConcreteDatabase, DATABASE_BACKEND};

mod constants;

mod database;
pub use database::Database;

mod error;
pub use error::{InitError, RuntimeError};

mod free;

mod macros;

mod pod;
pub use pod::{Pod, PodError};

mod table;
pub use table::Table;

mod transaction;
pub use transaction::{RoTx, RwTx};

//---------------------------------------------------------------------------------------------------- Private

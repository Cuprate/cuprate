//! `cuprate-database` testing & benchmarking binary.

//---------------------------------------------------------------------------------------------------- Lints
// Forbid lints.
// Our code, and code generated (e.g macros) cannot overrule these.
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
    clippy::undocumented_unsafe_blocks,
    unused_unsafe,
    redundant_semicolons,
    unused_allocation,
    coherence_leak_check,
    while_true,
    clippy::missing_docs_in_private_items,
    unconditional_recursion,
    for_loops_over_fallibles,
    unused_doc_comments,
    unused_labels,
    keyword_idents,
    non_ascii_idents,
    variant_size_differences,
    single_use_lifetimes,
    future_incompatible,
    let_underscore,
    break_with_label_and_loop,
    duplicate_macro_attributes,
    exported_private_dependencies,
    large_assignments,
    overlapping_range_endpoints,
    semicolon_in_expressions_from_macros,
    noop_method_call,
    unused_mut,
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

//---------------------------------------------------------------------------------------------------- Public API
// Import private modules, export public types.
//
// Documentation for each module is located in the respective file.
mod cli;
mod config;
mod constants;
mod free;

//---------------------------------------------------------------------------------------------------- Private

//---------------------------------------------------------------------------------------------------- Import
use crate::{cli::Cli, config::Config};

//---------------------------------------------------------------------------------------------------- Main
fn main() {
    // Handle CLI arguments.
    let config: Config = if std::env::args_os().len() > 1 {
        // Some arguments were passed, run all the `clap` code.
        Cli::init()
    } else {
        // No arguments were passed, use the default config.
        Config::default()
    };

    // // If `dry_run`, print config/stats/etc and exit cleanly.
    // if config.dry_run {
    //     println!("{}", serde_json::to_string_pretty(CONFIG).unwrap());
    //     std::process::exit(0);
    // }
}

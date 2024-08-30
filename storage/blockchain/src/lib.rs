#![doc = include_str!("../README.md")]
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
    unused_crate_dependencies,
    unused_doc_comments,
    unused_mut,
    //missing_docs,
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
#![cfg_attr(
    debug_assertions,
    allow(
        clippy::todo,
        clippy::multiple_crate_versions,
        // unused_crate_dependencies,
    )
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
extern crate core;

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

mod constants;
mod free;

pub use constants::DATABASE_VERSION;
pub use cuprate_database;
pub use free::open;

pub mod config;
pub mod ops;
pub mod tables;
pub mod types;

//---------------------------------------------------------------------------------------------------- Feature-gated
#[cfg(feature = "service")]
pub mod service;

//---------------------------------------------------------------------------------------------------- Private
#[cfg(test)]
pub(crate) mod tests;

#[cfg(feature = "service")] // only needed in `service` for now
pub(crate) mod unsafe_sendable;

#![doc = include_str!("../README.md")]
//---------------------------------------------------------------------------------------------------- Lints
#![allow(clippy::len_zero, clippy::type_complexity, clippy::module_inception)]
#![deny(nonstandard_style, deprecated, missing_docs, unused_mut)]
#![forbid(
    unused_unsafe,
    future_incompatible,
    break_with_label_and_loop,
    coherence_leak_check,
    duplicate_macro_attributes,
    exported_private_dependencies,
    for_loops_over_fallibles,
    large_assignments,
    overlapping_range_endpoints,
    // private_in_public,
    semicolon_in_expressions_from_macros,
    redundant_semicolons,
    unconditional_recursion,
    unreachable_patterns,
    unused_allocation,
    unused_braces,
    unused_comparisons,
    unused_doc_comments,
    unused_parens,
    unused_labels,
    while_true,
    keyword_idents,
    non_ascii_idents,
    noop_method_call,
	unreachable_pub,
    single_use_lifetimes,
	// variant_size_differences,
)]

//---------------------------------------------------------------------------------------------------- Public API
pub mod asynch; // async collides
pub mod crypto;
pub mod num;
pub mod time;

//---------------------------------------------------------------------------------------------------- Private Usage

//----------------------------------------------------------------------------------------------------

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

//---------------------------------------------------------------------------------------------------- Use
// Misc types.
mod status;
pub use status::Status;

// Internal modules.
mod macros;

// Request/response JSON/binary/other types.
mod bin;
mod json;
mod other;

/// TODO
///
/// TODO: explain
/// - how this works
/// - where to add types
/// - when to add
/// - what to do when adding/editing types
macro_rules! re_export_request_and_response_types {
    (
		json {
			$(
				$json_type:ident,
			)*
		}
		bin {
			$(
				$binary_type:ident,
			)*
		}
		other {
			$(
				$other_type:ident,
			)*
		}
	) => { paste::paste! {
		/// RPC request types.
		pub mod req {
			/// JSON types from the [`/json_rpc`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#json-rpc-methods) endpoint.
			pub mod json {
				$(
					pub use $crate::json::[<Request $json_type>] as $json_type;
				)*
			}

			/// Binary types from [binary](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_blocksbin) endpoints.
			pub mod bin {
				$(
					pub use $crate::bin::[<Request $binary_type>] as $binary_type;
				)*
			}

			/// JSON types from the [`other`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#other-daemon-rpc-calls) endpoints.
			pub mod other {
				$(
					pub use $crate::other::[<Request $other_type>] as $other_type;
				)*
			}
		}

		/// RPC response types.
		pub mod resp {
			/// JSON types from the [`/json_rpc`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#json-rpc-methods) endpoint.
			pub mod json {
				$(
					pub use $crate::json::[<Response $json_type>] as $json_type;
				)*
			}

			/// Binary types from [binary](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_blocksbin) endpoints.
			pub mod bin {
				$(
					pub use $crate::bin::[<Response $binary_type>] as $binary_type;
				)*
			}

			/// JSON types from the [`other`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#other-daemon-rpc-calls) endpoints.
			pub mod other {
				$(
					pub use $crate::other::[<Response $other_type>] as $other_type;
				)*
			}
		}
	}};
}

re_export_request_and_response_types! {
    json {
        GetBlockCount,
        OnGetBlockHash,
        GetBlockTemplate,
    }

    bin {
    }

    other {
        SaveBc,
    }
}

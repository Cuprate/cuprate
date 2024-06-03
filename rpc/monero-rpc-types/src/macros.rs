//! Macros.
//!
//! These generate repetitive documentation, tests, etc.

//---------------------------------------------------------------------------------------------------- Struct definition
/// A template for generating a `struct` with a bunch of information filled out.
///
/// It's best to see the output of this macro via the documentation
/// of the generated structs via `cargo doc`s to see which parts
/// generate which docs.
///
/// See [`crate::json::GetHeight`] for example usage.
macro_rules! define_monero_rpc_type_struct {
    (
        // The markdown tag for Monero RPC documentation. Not necessarily the endpoint.
        $monero_daemon_rpc_doc_link:ident,

        // The `$file.$extension` in which this type is defined in the Monero
        // codebase in the `rpc/` directory, followed by the specific lines.
        $monero_code_filename:ident.$monero_code_filename_extension:ident =>
        $monero_code_line_start:literal..=$monero_code_line_end:literal,

        // A real literal type expression, and its JSON string form.
        // Used in example doc-test.
        $type_literal:expr => $type_as_json:literal,

        // The actual `struct` name and any doc comments, derives, etc.
        $( #[$type_attr:meta] )*
        $type_name:ident {
            // And any fields.
            $(
                $( #[$field_attr:meta] )*
                $field:ident: $field_type:ty,
            )*
        }
    ) => {
        $( #[$type_attr:meta] )*
        #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        ///
        #[doc = concat!(
            "",
            "Definition: [`",
            stringify!($monero_code_filename),
            ".",
            stringify!($monero_code_filename_extension),
            " @ ",
            stringify!($monero_code_line_start),
            "..=",
            stringify!($monero_code_line_end),
            "`](",
            "https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/",
            stringify!($monero_code_filename),
            ".",
            stringify!($monero_code_filename_extension),
            "#L",
            stringify!($monero_code_line_start),
            "-L",
            stringify!($monero_code_line_end),
            "), documentation: [`",
            stringify!($monero_daemon_rpc_doc_link),
            "`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html",
            "#",
            stringify!($monero_daemon_rpc_doc_link),
            ").",
        )]
        ///
        /// # `serde` example
        /// ```rust
        #[doc = "# use monero_rpc_types::{json::*, binary::*, data::*, misc::*, other::*};"]
        #[doc = concat!("let t = ", stringify!($type_literal), ";")]
        #[doc = "let string = serde_json::to_string(&t).unwrap();"]
        #[doc = concat!("assert_eq!(string, ", stringify!($type_as_json), ");")]
        #[doc = ""]
        #[doc = "let t2 = serde_json::from_str(&string).unwrap();"]
        #[doc = "assert_eq!(t, t2);"]
        /// ```
        pub struct $type_name {
            $(
                $( #[$field_attr] )*
                pub $field: $field_type,
            )*
        }
    };
}
pub(crate) use define_monero_rpc_type_struct;

//---------------------------------------------------------------------------------------------------- Documentation macros

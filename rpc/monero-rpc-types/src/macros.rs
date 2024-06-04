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
macro_rules! define_monero_rpc_struct {
    (
        // The markdown tag for Monero RPC documentation. Not necessarily the endpoint.
        $monero_daemon_rpc_doc_link:ident,

        // The `$file.$extension` in which this type is defined in the Monero
        // codebase in the `rpc/` directory, followed by the specific lines.
        $monero_code_filename:ident.$monero_code_filename_extension:ident =>
        $monero_code_line_start:literal..=$monero_code_line_end:literal,

        // The actual request `struct` name and any doc comments, derives, etc.
        $( #[$request_type_attr:meta] )*
        $type_name:ident,
        Request {
            // And any fields.
            $(
                $( #[$request_field_attr:meta] )*
                $request_field:ident: $request_field_type:ty,
            )*
        },

        // The actual `struct` name and any doc comments, derives, etc.
        $( #[$response_type_attr:meta] )*
        Response {
            // And any fields.
            $(
                $( #[$response_field_attr:meta] )*
                $response_field:ident: $response_field_type:ty,
            )*
        }
    ) => { paste::paste! {
        #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        $( #[$request_type_attr] )*
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
            ")."
        )]
        #[allow(dead_code)]
        pub struct [<Request $type_name>] {
            $(
                $( #[$request_field_attr] )*
                pub $request_field: $request_field_type,
            )*
        }

        #[allow(dead_code)]
        /// TODO
        pub struct [<Response $type_name>] {
            $(
                $( #[$response_field_attr] )*
                pub $response_field: $response_field_type,
            )*
        }
    }};
}
pub(crate) use define_monero_rpc_struct;

//---------------------------------------------------------------------------------------------------- Documentation macros

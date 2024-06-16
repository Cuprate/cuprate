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
        $type_name:ident,
        $( #[$request_type_attr:meta] )*
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
        #[allow(dead_code)]
        #[allow(missing_docs)]
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $( #[$request_type_attr] )*
        #[doc = concat!(
            "",
            "[Definition](",
            "https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/",
            stringify!($monero_code_filename),
            ".",
            stringify!($monero_code_filename_extension),
            "#L",
            stringify!($monero_code_line_start),
            "-L",
            stringify!($monero_code_line_end),
            "), [documentation](",
            "https://www.getmonero.org/resources/developer-guides/daemon-rpc.html",
            "#",
            stringify!($monero_daemon_rpc_doc_link),
            "), [response](",
            stringify!([<$type_name Response>]),
            ")."
        )]
        pub struct [<$type_name Request>] {
            $(
                $( #[$request_field_attr] )*
                pub $request_field: $request_field_type,
            )*
        }

        #[allow(dead_code)]
        #[allow(missing_docs)]
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $( #[$response_type_attr] )*
        #[doc = concat!(
            "",
            "[Definition](",
            "https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/",
            stringify!($monero_code_filename),
            ".",
            stringify!($monero_code_filename_extension),
            "#L",
            stringify!($monero_code_line_start),
            "-L",
            stringify!($monero_code_line_end),
            "), [documentation](",
            "https://www.getmonero.org/resources/developer-guides/daemon-rpc.html",
            "#",
            stringify!($monero_daemon_rpc_doc_link),
            "), [request](",
            stringify!([<$type_name Request>]),
            ")."
        )]
        pub struct [<$type_name Response>] {
            $(
                $( #[$response_field_attr] )*
                pub $response_field: $response_field_type,
            )*
        }
    }};
}
pub(crate) use define_monero_rpc_struct;

//---------------------------------------------------------------------------------------------------- Documentation macros

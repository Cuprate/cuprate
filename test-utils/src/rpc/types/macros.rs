//! Macros.

//---------------------------------------------------------------------------------------------------- define_request_and_response
/// A template for generating the RPC request and response `const` data.
///
/// See the [`crate::json`] module for example usage.
///
/// # Macro internals
/// This macro uses:
/// - [`define_request_and_response_doc`]
/// - [`define_request_and_response_test`]
macro_rules! define_request_and_response {
    (
        // The markdown tag for Monero daemon RPC documentation. Not necessarily the endpoint.
        //
        // Adding `(json)` after this will trigger the macro to automatically
        // add a `serde_json` test for the request/response data.
        $monero_daemon_rpc_doc_link:ident $(($json:ident))?,

        // The base `struct` name.
        // Attributes added here will apply to _both_
        // request and response types.
        $( #[$attr:meta] )*
        $name:ident: $type:ty,

        // The request type (and any doc comments, derives, etc).
        $( #[$request_attr:meta] )*
        Request = $request:literal;

        // The response type (and any doc comments, derives, etc).
        $( #[$response_attr:meta] )*
        Response = $response:literal;
    ) => { paste::paste! {
        #[doc = $crate::rpc::types::macros::define_request_and_response_doc!(
            "response" => [<$name:upper _RESPONSE>],
            $monero_daemon_rpc_doc_link,
        )]
        ///
        $( #[$attr] )*
        ///
        $( #[$request_attr] )*
        ///
        $(
            const _: &str = stringify!($json);
            #[doc = $crate::rpc::types::macros::json_test!([<$name:upper _REQUEST>])]
        )?
        pub const [<$name:upper _REQUEST>]: $type = $request;

        #[doc = $crate::rpc::types::macros::define_request_and_response_doc!(
            "request" => [<$name:upper _REQUEST>],
            $monero_daemon_rpc_doc_link,
        )]
        ///
        $( #[$attr] )*
        ///
        $( #[$response_attr] )*
        ///
        $(
            const _: &str = stringify!($json);
            #[doc = $crate::rpc::types::macros::json_test!([<$name:upper _RESPONSE>])]
        )?
        pub const [<$name:upper _RESPONSE>]: $type = $response;
    }};
}
pub(super) use define_request_and_response;

//---------------------------------------------------------------------------------------------------- define_request_and_response_doc
/// Generate documentation for the types generated
/// by the [`define_request_and_response`] macro.
///
/// See it for more info on inputs.
macro_rules! define_request_and_response_doc {
    (
        // This labels the last `[request]` or `[response]`
        // hyperlink in documentation. Input is either:
        // - "request"
        // - "response"
        //
        // Remember this is linking to the _other_ type,
        // so if defining a `Request` type, input should
        // be "response".
        $request_or_response:literal => $request_or_response_type:ident,
        $monero_daemon_rpc_doc_link:ident,
    ) => {
        concat!(
            "",
            "[Documentation](",
            "https://www.getmonero.org/resources/developer-guides/daemon-rpc.html",
            "#",
            stringify!($monero_daemon_rpc_doc_link),
            "), [",
            $request_or_response,
            "](",
            stringify!($request_or_response_type),
            ")."
        )
    };
}
pub(super) use define_request_and_response_doc;

//---------------------------------------------------------------------------------------------------- define_request_and_response_test
/// Generate documentation for the types generated
/// by the [`define_request_and_response`] macro.
///
/// See it for more info on inputs.
macro_rules! json_test {
    (
        $name:ident // TODO
    ) => {
        concat!(
            "```rust",
            "use cuprate_test_utils::rpc::types::{json::*,bin::*,other::*};",
            "use serde_json::to_value;",
            "",
            "let value = serde_json::to_value(&",
            stringify!($name),
            ").unwrap();",
            "let string = serde_json::to_string_pretty(&value).unwrap();",
            "assert_eq!(string, ",
            stringify!($name),
            ");",
            "```",
        )
    };
}
pub(super) use json_test;

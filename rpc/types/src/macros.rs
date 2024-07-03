//! Macros.

//---------------------------------------------------------------------------------------------------- define_request_and_response
/// A template for generating the RPC request and response `struct`s.
///
/// These `struct`s automatically implement:
/// - `Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash`
/// - `serde::{Serialize, Deserialize}`
/// - `cuprate_epee_encoding::EpeeObject`
///
/// It's best to see the output of this macro via the documentation
/// of the generated structs via `cargo doc`s to see which parts
/// generate which docs.
///
/// See the [`crate::json`] module for example usage.
///
/// # Macro internals
/// This macro has 2 branches with almost the same output:
/// 1. An empty `Request` type
/// 2. An `Request` type with fields
///
/// The first branch is the same as the second with the exception
/// that if the caller of this macro provides no `Request` type,
/// it will generate:
/// ```
/// pub type Request = ();
/// ```
/// instead of:
/// ```
/// pub struct Request {/* fields */}
/// ```
///
/// This is because having a bunch of types that are all empty structs
/// means they are not compatible and it makes it cumbersome for end-users.
/// Really, they semantically are empty types, so `()` is used.
///
/// Again, other than this, the 2 branches do (should) not differ.
///
/// FIXME: there's probably a less painful way to branch here on input
/// without having to duplicate 80% of the macro. Sub-macros were attempted
/// but they ended up unreadable. So for now, make sure to fix the other
/// branch as well when making changes. The only de-duplicated part is
/// the doc generation with [`define_request_and_response_doc`].
macro_rules! define_request_and_response {
    (
        // The markdown tag for Monero RPC documentation. Not necessarily the endpoint.
        $monero_daemon_rpc_doc_link:ident,

        // The commit hash and  `$file.$extension` in which this type is defined in
        // the Monero codebase in the `rpc/` directory, followed by the specific lines.
        $monero_code_commit:ident =>
        $monero_code_filename:ident.
        $monero_code_filename_extension:ident =>
        $monero_code_line_start:literal..=
        $monero_code_line_end:literal,

        // The base `struct` name.
        $type_name:ident,

        // The request type (and any doc comments, derives, etc).
        $( #[$request_type_attr:meta] )*
        Request {
            // And any fields.
            $(
                $( #[$request_field_attr:meta] )*
                $request_field:ident: $request_field_type:ty,
            )*
        },

        // The response type (and any doc comments, derives, etc).
        $( #[$response_type_attr:meta] )*
        $response_base_type:ty {
            // And any fields.
            $(
                $( #[$response_field_attr:meta] )*
                $response_field:ident: $response_field_type:ty,
            )*
        }
    ) => { paste::paste! {
        $crate::macros::define_request! {
            #[doc = $crate::macros::define_request_and_response_doc!(
                "response" => [<$type_name Response>],
                $monero_daemon_rpc_doc_link,
                $monero_code_commit,
                $monero_code_filename,
                $monero_code_filename_extension,
                $monero_code_line_start,
                $monero_code_line_end,
            )]
            $( #[$request_type_attr] )*
            [<$type_name Request>] {
                $(
                    $( #[$request_field_attr] )*
                    $request_field: $request_field_type,
                )*
            }
        }

        $crate::macros::define_response! {
            #[allow(dead_code)]
            #[allow(missing_docs)]
            #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
            #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #[doc = $crate::macros::define_request_and_response_doc!(
                "request" => [<$type_name Request>],
                $monero_daemon_rpc_doc_link,
                $monero_code_commit,
                $monero_code_filename,
                $monero_code_filename_extension,
                $monero_code_line_start,
                $monero_code_line_end,
            )]
            $( #[$response_type_attr] )*
            $response_base_type => [<$type_name Response>] {
                $(
                    $( #[$response_field_attr] )*
                    $response_field: $response_field_type,
                )*
            }
        }
    }};
}
pub(crate) use define_request_and_response;

//---------------------------------------------------------------------------------------------------- define_request
/// TODO
macro_rules! define_request {
    (
        // The response type (and any doc comments, derives, etc).
        $( #[$request_type_attr:meta] )*
        $request_type_name:ident {}
    ) => {
        $( #[$request_type_attr] )*
        ///
        /// This request has no inputs.
        pub type $request_type_name = ();
    };

    //------------------------------------------------------------------------------
    // This version of the macro expects a `Request` base type.
    (
        // The response type (and any doc comments, derives, etc).
        $( #[$request_type_attr:meta] )*
        $request_type_name:ident {
            // And any fields.
            $(
                $( #[$request_field_attr:meta] )*
                $request_field:ident: $request_field_type:ty,
            )*
        }
    ) => {
        #[allow(dead_code, missing_docs)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $( #[$request_type_attr] )*
        pub struct $request_type_name {
            $(
                $( #[$request_field_attr] )*
                pub $request_field: $request_field_type,
            )*
        }

        #[cfg(feature = "epee")]
        ::cuprate_epee_encoding::epee_object! {
            $request_type_name,
            $(
                $request_field: $request_field_type,
            )*
        }
    };
}
pub(crate) use define_request;

//---------------------------------------------------------------------------------------------------- define_response
/// TODO
macro_rules! define_response {
    (
        // The response type (and any doc comments, derives, etc).
        $( #[$response_type_attr:meta] )*
        Response => $response_type_name:ident {
            // And any fields.
            $(
                $( #[$response_field_attr:meta] )*
                $response_field:ident: $response_field_type:ty,
            )*
        }
    ) => {
        $( #[$response_type_attr] )*
        pub struct $response_type_name {
            $(
                $( #[$response_field_attr] )*
                pub $response_field: $response_field_type,
            )*
        }

        #[cfg(feature = "epee")]
        ::cuprate_epee_encoding::epee_object! {
            $response_type_name,
            $(
                $response_field: $response_field_type,
            )*
        }
    };

    //------------------------------------------------------------------------------
    // This version of the macro expects a `Request` base type.
    (
        // The response type (and any doc comments, derives, etc).
        $( #[$response_type_attr:meta] )*
        $response_base_type:ty => $response_type_name:ident {
            // And any fields.
            $(
                $( #[$response_field_attr:meta] )*
                $response_field:ident: $response_field_type:ty,
            )*
        }
    ) => {
        $( #[$response_type_attr] )*
        pub struct $response_type_name {
            #[cfg_attr(feature = "serde", serde(flatten))]
            pub base: $response_base_type,

            $(
                $( #[$response_field_attr] )*
                pub $response_field: $response_field_type,
            )*
        }

        #[cfg(feature = "epee")]
        ::cuprate_epee_encoding::epee_object! {
            $response_type_name,
            $(
                $response_field: $response_field_type,
            )*
            !flatten: base: $response_base_type,
        }
    };
}
pub(crate) use define_response;

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
        $monero_code_commit:ident,
        $monero_code_filename:ident,
        $monero_code_filename_extension:ident,
        $monero_code_line_start:literal,
        $monero_code_line_end:literal,
    ) => {
        concat!(
            "",
            "[Definition](",
            "https://github.com/monero-project/monero/blob/",
            stringify!($monero_code_commit),
            "/src/rpc/",
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
            "), [",
            $request_or_response,
            "](",
            stringify!($request_or_response_type),
            ")."
        )
    };
}
pub(crate) use define_request_and_response_doc;

//---------------------------------------------------------------------------------------------------- Macro
/// Link the original `monerod` definition for RPC base types.
macro_rules! monero_definition_link {
    ($commit:ident => $file:ident.$file_extension:ident => $start:literal..=$end:literal) => {
        concat!(
            "[Definition](https://github.com/monero-project/monero/blob/",
            stringify!($commit),
            "/src/rpc/",
            stringify!($file),
            ".",
            stringify!($file_extension),
            "#L",
            stringify!($start),
            "-L",
            stringify!($end),
            ")."
        )
    };
}
pub(crate) use monero_definition_link;

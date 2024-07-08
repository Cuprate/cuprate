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
/// This macro uses:
/// - [`__define_request`]
/// - [`__define_response`]
/// - [`__define_request_and_response_doc`]
///
/// # `__define_request`
/// This macro has 2 branches. If the caller provides
/// `Request {}`, i.e. no fields, it will generate:
/// ```
/// pub type Request = ();
/// ```
/// If they _did_ specify fields, it will generate:
/// ```
/// pub struct Request {/* fields */}
/// ```
/// This is because having a bunch of types that are all empty structs
/// means they are not compatible and it makes it cumbersome for end-users.
/// Really, they semantically are empty types, so `()` is used.
///
/// # `__define_response`
/// This macro has 2 branches. If the caller provides `Response`
/// it will generate a normal struct with no additional fields.
///
/// If the caller provides a base type from [`crate::base`], it will
/// flatten that into the request type automatically.
///
/// E.g. `Response {/*...*/}` and `ResponseBase {/*...*/}`
/// would trigger the different branches.
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
        // Attributes added here will apply to _both_
        // request and response types.
        $( #[$type_attr:meta] )*
        $type_name:ident,

        // The request type (and any doc comments, derives, etc).
        $( #[$request_type_attr:meta] )*
        Request {
            // And any fields.
            $(
                $( #[$request_field_attr:meta] )*
                $request_field:ident: $request_field_type:ty $(= $request_field_type_default:expr)?,
            )*
        },

        // The response type (and any doc comments, derives, etc).
        $( #[$response_type_attr:meta] )*
        $response_base_type:ty {
            // And any fields.
            $(
                $( #[$response_field_attr:meta] )*
                $response_field:ident: $response_field_type:ty $(= $response_field_type_default:expr)?,
            )*
        }
    ) => { paste::paste! {
        $crate::macros::__define_request! {
            #[doc = $crate::macros::__define_request_and_response_doc!(
                "response" => [<$type_name Response>],
                $monero_daemon_rpc_doc_link,
                $monero_code_commit,
                $monero_code_filename,
                $monero_code_filename_extension,
                $monero_code_line_start,
                $monero_code_line_end,
            )]
            ///
            $( #[$type_attr] )*
            ///
            $( #[$request_type_attr] )*
            [<$type_name Request>] {
                $(
                    $( #[$request_field_attr] )*
                    $request_field: $request_field_type $(= $request_field_type_default)?,
                )*
            }
        }

        $crate::macros::__define_response! {
            #[allow(dead_code)]
            #[allow(missing_docs)]
            #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
            #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #[doc = $crate::macros::__define_request_and_response_doc!(
                "request" => [<$type_name Request>],
                $monero_daemon_rpc_doc_link,
                $monero_code_commit,
                $monero_code_filename,
                $monero_code_filename_extension,
                $monero_code_line_start,
                $monero_code_line_end,
            )]
            ///
            $( #[$type_attr] )*
            ///
            $( #[$response_type_attr] )*
            $response_base_type => [<$type_name Response>] {
                $(
                    $( #[$response_field_attr] )*
                    $response_field: $response_field_type $(= $response_field_type_default)?,
                )*
            }
        }
    }};
}
pub(crate) use define_request_and_response;

//---------------------------------------------------------------------------------------------------- define_request
/// Define a request type.
///
/// This is only used in [`define_request_and_response`], see it for docs.
///
/// `__` is used to notate that this shouldn't be called directly.
macro_rules! __define_request {
    //------------------------------------------------------------------------------
    // This branch will generate a type alias to `()` if only given `{}` as input.
    (
        // Any doc comments, derives, etc.
        $( #[$attr:meta] )*
        // The response type.
        $t:ident {}
    ) => {
        $( #[$attr] )*
        ///
        /// This request has no inputs.
        pub type $t = ();
    };

    //------------------------------------------------------------------------------
    // This branch of the macro expects fields within the `{}`,
    // and will generate a `struct`
    (
        // Any doc comments, derives, etc.
        $( #[$attr:meta] )*
        // The response type.
        $t:ident {
            // And any fields.
            $(
                $( #[$field_attr:meta] )* // field attributes
                // field_name: FieldType
                $field:ident: $field_type:ty $(= $field_default:expr)?,
                // The $field_default is an optional extra token that represents
                // a default value to pass to [`cuprate_epee_encoding::epee_object`],
                // see it for usage.
            )*
        }
    ) => {
        #[allow(dead_code, missing_docs)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $( #[$attr] )*
        pub struct $t {
            $(
                $( #[$field_attr] )*
                pub $field: $field_type,
            )*
        }

        #[cfg(feature = "epee")]
        ::cuprate_epee_encoding::epee_object! {
            $t,
            $(
                $field: $field_type $(= $field_default)?,
            )*
        }
    };
}
pub(crate) use __define_request;

//---------------------------------------------------------------------------------------------------- define_response
/// Define a response type.
///
/// This is only used in [`define_request_and_response`], see it for docs.
///
/// `__` is used to notate that this shouldn't be called directly.
macro_rules! __define_response {
    //------------------------------------------------------------------------------
    // This version of the macro expects the literal ident
    // `Response` => $response_type_name.
    //
    // It will create a `struct` that _doesn't_ use a base from [`crate::base`],
    // for example, [`crate::json::BannedResponse`] doesn't use a base, so it
    // uses this branch.
    (
        // Any doc comments, derives, etc.
        $( #[$attr:meta] )*
        // The response type.
        Response => $t:ident {
            // And any fields.
            // See [`__define_request`] for docs, this does the same thing.
            $(
                $( #[$field_attr:meta] )*
                $field:ident: $field_type:ty $(= $field_default:expr)?,
            )*
        }
    ) => {
        $( #[$attr] )*
        pub struct $t {
            $(
                $( #[$field_attr] )*
                pub $field: $field_type,
            )*
        }

        #[cfg(feature = "epee")]
        ::cuprate_epee_encoding::epee_object! {
            $t,
            $(
                $field: $field_type $($field_default)?,
            )*
        }
    };

    //------------------------------------------------------------------------------
    // This version of the macro expects a `Request` base type from [`crate::bases`].
    (
        // Any doc comments, derives, etc.
        $( #[$attr:meta] )*
        // The response base type => actual name of the struct
        $base:ty => $t:ident {
            // And any fields.
            // See [`__define_request`] for docs, this does the same thing.
            $(
                $( #[$field_attr:meta] )*
                $field:ident: $field_type:ty $(= $field_default:expr)?,
            )*
        }
    ) => {
        $( #[$attr] )*
        pub struct $t {
            #[cfg_attr(feature = "serde", serde(flatten))]
            pub base: $base,

            $(
                $( #[$field_attr] )*
                pub $field: $field_type,
            )*
        }

        #[cfg(feature = "epee")]
        ::cuprate_epee_encoding::epee_object! {
            $t,
            $(
                $field: $field_type $(= $field_default)?,
            )*
            !flatten: base: $base,
        }
    };
}
pub(crate) use __define_response;

//---------------------------------------------------------------------------------------------------- define_request_and_response_doc
/// Generate documentation for the types generated
/// by the [`__define_request_and_response`] macro.
///
/// See it for more info on inputs.
///
/// `__` is used to notate that this shouldn't be called directly.
macro_rules! __define_request_and_response_doc {
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
pub(crate) use __define_request_and_response_doc;

//---------------------------------------------------------------------------------------------------- Macro
/// Output a string link to `monerod` source code.
macro_rules! monero_definition_link {
    (
        $commit:ident, // Git commit hash
        $file_path:literal, // File path within `monerod`'s `src/`, e.g. `rpc/core_rpc_server_commands_defs.h`
        $start:literal$(..=$end:literal)? // File lines, e.g. `0..=123` or `0`
    ) => {
        concat!(
            "[Definition](https://github.com/monero-project/monero/blob/",
            stringify!($commit),
            "/src/",
            $file_path,
            "#L",
            stringify!($start),
            $(
                "-L",
                stringify!($end),
            )?
            ")."
        )
    };
}
pub(crate) use monero_definition_link;

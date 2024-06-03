//! Macros.
//!
//! These generate repetitive documentation, tests, etc.

//---------------------------------------------------------------------------------------------------- Struct definition
/// TODO
macro_rules! define_monero_rpc_type_struct {
    (
        $monero_daemon_rpc_doc_link:ident, // TODO
        $monero_code_filename:ident.$monero_code_filename_extension:ident => // TODO
        $monero_code_line_start:literal..= // TODO
        $monero_code_line_end:literal, // TODO
        $type_literal:expr => // TODO
        $type_as_json:literal, // TODO
        $type_name:ident { // TODO
            $(
                $( #[$field_attr:meta] )* // TODO
                $field:ident: $type:ty, // TODO
            )*
        }
    ) => {
        ///
        #[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        ///
        // Documents the original Monero type and where it is located in `monero-project/monero`.
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
        // Doc-test that tests (de)serialization.
        /// # `serde` example
        /// ```rust
        #[doc = "# use monero_rpc_types::{json::*, binary::*, data::*, mix::*};"]
        #[doc = concat!("let t = ", stringify!($type_literal), ";")]
        #[doc = "let string = serde_json::to_string(&t).unwrap();"]
        #[doc = concat!("assert_eq!(string, ", stringify!($type_as_json), ");")]
        #[doc = ""]
        #[doc = "let t2 = serde_json::from_str(&string).unwrap();"]
        #[doc = "assert_eq!(t, t2);"]
        /// ```
        // The type.
        pub struct $type_name {
            $(
                $( #[$field_attr] )*
                pub $field: $type,
            )*
        }
    };
}
pub(crate) use define_monero_rpc_type_struct;

//---------------------------------------------------------------------------------------------------- Documentation macros

//! Other endpoint types.
//!
//! <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/daemon_messages.h>.

//---------------------------------------------------------------------------------------------------- Import
use crate::macros::define_monero_rpc_type_struct;

//---------------------------------------------------------------------------------------------------- TODO
define_monero_rpc_type_struct! {
    // The markdown tag for Monero RPC documentation. Not necessarily the endpoint.
    get_height,
    // The `$file.$extension` in which this type is defined in the Monero
    // codebase in the `rpc/` directory, followed by the specific lines.
    daemon_messages.h => 81..=87,
    // The type and its compacted JSON string form, used in example doc-test.
    GetHeight { height: 123 } => r#"{"height":123}"#,
    // The actual type definitions.
    // If there are any additional attributes (`/// docs` or `#[derive]`s)
    // for the struct, they go here, e.g.:
    // #[derive(MyCustomDerive)]
    GetHeight /* <- The type name */ {
        // Within the `{}` is an infinite matching pattern of:
        // ```
        // $ATTRIBUTES
        // $FIELD_NAME: $FIELD_TYPE,
        // ```
        // The struct generated and all fields are `pub`.

        /// A block height.
        height: u64,
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

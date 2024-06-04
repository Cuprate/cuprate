//! JSON types.
//!
//! <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/daemon_messages.h>.

//---------------------------------------------------------------------------------------------------- Import
use crate::{macros::define_monero_rpc_struct, misc::Status};

//---------------------------------------------------------------------------------------------------- Struct definitions
define_monero_rpc_struct! {
    // The markdown tag for Monero RPC documentation. Not necessarily the endpoint.
    get_block_count,
    // The `$file.$extension` in which this type is defined in the Monero
    // codebase in the `rpc/` directory, followed by the specific lines.
    core_rpc_server_commands_defs.h => 919..=933,
    // The type and its compacted JSON string form, used in example doc-test.
    GetBlockCount { count: 123, status: Status::Ok, untrusted: false } =>
    r#"{"count":123,"status":"OK","untrusted":false}"#,
    // The actual type definitions.
    // If there are any additional attributes (`/// docs` or `#[derive]`s)
    // for the struct, they go here, e.g.:
    // #[derive(MyCustomDerive)]
    GetBlockCount /* <- The type name */ {
        // Within the `{}` is an infinite matching pattern of:
        // ```
        // $ATTRIBUTES
        // $FIELD_NAME: $FIELD_TYPE,
        // ```
        // The struct generated and all fields are `pub`.

        /// How many blocks are in the longest chain known to the node.
        count: u64,
        /// General RPC error code. "OK" means everything looks good.
        status: Status,
        /// Whether the node is untrusted (see Monero docs).
        untrusted: bool,
    }
}

define_monero_rpc_struct! {
    on_get_block_hash,
    core_rpc_server_commands_defs.h => 919..=933,
    OnGetBlockHash { height: [123] } =>
    r#"[123]"#,
    #[repr(transparent)]
    #[cfg_attr(feature = "serde", serde(transparent))]
    OnGetBlockHash {
        /// A block's height.
        height: [u64; 1],
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

//! JSON types from the [`/json_rpc`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#json-rpc-methods) endpoint.
//!
//! <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/daemon_messages.h>.

//---------------------------------------------------------------------------------------------------- Import
use crate::macros::define_monero_rpc_struct;

//---------------------------------------------------------------------------------------------------- Struct definitions
define_monero_rpc_struct! {
    // The markdown tag for Monero RPC documentation. Not necessarily the endpoint.
    get_block_count,
    // The `$file.$extension` in which this type is defined in the Monero
    // codebase in the `rpc/` directory, followed by the specific lines.
    core_rpc_server_commands_defs.h => 919..=933,
    // The actual type definitions.
    // If there are any additional attributes (`/// docs` or `#[derive]`s)
    // for the struct, they go here, e.g.:
    // #[derive(MyCustomDerive)]
    GetBlockCount, // <- The type name.
    #[derive(Copy)]
    Request /* <- The request type */ {
        // This request type requires no inputs,
        // so it is left empty.
    },
    #[derive(Copy)]
    Response /* <- The response type */ {
        // Within the `{}` is an infinite matching pattern of:
        // ```
        // $ATTRIBUTES
        // $FIELD_NAME: $FIELD_TYPE,
        // ```
        // The struct generated and all fields are `pub`.

        count: u64,
        status: crate::Status,
        untrusted: bool,
    }
}

define_monero_rpc_struct! {
    on_get_block_hash,
    core_rpc_server_commands_defs.h => 935..=939,
    OnGetBlockHash,
    #[derive(Copy)]
    Request {
        block_height: u64,
    },
    Response {
        block_hash: String,
    }
}

define_monero_rpc_struct! {
    get_block_template,
    core_rpc_server_commands_defs.h => 943..=994,
    GetBlockTemplate,
    Request {
        reserve_size: u64,
        wallet_address: String,
    },
    Response {
        difficulty: u64,
        wide_difficulty: String,
        difficulty_top64: u64,
        height: u64,
        reserved_offset: u64,
        expected_reward: u64,
        prev_hash: String,
        seed_height: u64,
        seed_hash: String,
        next_seed_hash: String,
        blocktemplate_blob: String,
        blockhashing_blob: String,
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

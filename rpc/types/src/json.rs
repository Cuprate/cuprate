//! JSON types from the [`/json_rpc`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#json-rpc-methods) endpoint.
//!
//! <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/daemon_messages.h>.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    base::{EmptyRequestBase, EmptyResponseBase, ResponseBase},
    macros::define_request_and_response,
};

//---------------------------------------------------------------------------------------------------- Struct definitions
// This generates 2 structs:
//
// - `GetBlockCountRequest`
// - `GetBlockCountResponse`
//
// with some interconnected documentation.
define_request_and_response! {
    // The markdown tag for Monero RPC documentation. Not necessarily the endpoint.
    get_block_count,

    // The commit hash and `$file.$extension` in which this type is defined in
    // the Monero codebase in the `rpc/` directory, followed by the specific lines.
    cc73fe71162d564ffda8e549b79a350bca53c454 => core_rpc_server_commands_defs.h => 919..=933,

    // The base type name.
    GetBlockCount,

    // The request type.
    RequestBase {
        // This request type requires no inputs,
        // so it is left empty. Leaving this empty
        // will cause the macro to generate a type
        // alias to `()` instead of a `struct`.
    },

    // The response type.
    //
    // If there are any additional attributes (`/// docs` or `#[derive]`s)
    // for the struct, they go here, e.g.:
    // #[derive(Copy)]
    ResponseBase {
        // Within the `{}` is an infinite matching pattern of:
        // ```
        // $ATTRIBUTES
        // $FIELD_NAME: $FIELD_TYPE,
        // ```
        // The struct generated and all fields are `pub`.
        count: u64,
    }
}

define_request_and_response! {
    on_get_block_hash,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 935..=939,
    OnGetBlockHash,
    #[derive(Copy)]
    EmptyRequestBase {
        #[serde(flatten)]
        block_height: u64,
    },
    EmptyResponseBase {
        #[serde(flatten)]
        block_hash: String,
    }
}

define_request_and_response! {
    get_block_template,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 943..=994,
    GetBlockTemplate,
    EmptyRequestBase {
        reserve_size: u64,
        wallet_address: String,
    },
    ResponseBase {
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

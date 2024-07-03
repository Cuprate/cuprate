//! JSON types from the [`/json_rpc`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#json-rpc-methods) endpoint.
//!
//! <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/daemon_messages.h>.

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    base::{AccessResponseBase, ResponseBase},
    defaults::{default_bool, default_height, default_string},
    macros::define_request_and_response,
    misc::{BlockHeader, ConnectionInfo, GetBan, SetBan},
};

//---------------------------------------------------------------------------------------------------- Struct definitions
// This generates 2 structs:
//
// - `GetBlockTemplateRequest`
// - `GetBlockTemplateResponse`
//
// with some interconnected documentation.
define_request_and_response! {
    // The markdown tag for Monero RPC documentation. Not necessarily the endpoint.
    get_block_template,

    // The commit hash and `$file.$extension` in which this type is defined in
    // the Monero codebase in the `rpc/` directory, followed by the specific lines.
    cc73fe71162d564ffda8e549b79a350bca53c454 => core_rpc_server_commands_defs.h => 943..=994,

    // The base type name.
    GetBlockTemplate,

    // The base request type.
    //
    // This must be a type found in [`crate::base`].
    // It acts as a "base" that gets flattened into
    // the actual request type.
    //
    // "Flatten" means the field(s) of a struct gets inlined
    // directly into the struct during (de)serialization, see:
    // <https://serde.rs/field-attrs.html#flatten>.
    //
    // For example here, we're using [`crate::base::EmptyRequestBase`],
    // which means that there is no extra fields flattened.
    //
    // If a request is not specified here, it will create a `type YOUR_REQUEST_TYPE = ()`
    // instead of a `struct`, see below in other macro definitions for an example.
    Request {
        // Within the `{}` is an infinite matching pattern of:
        // ```
        // $ATTRIBUTES
        // $FIELD_NAME: $FIELD_TYPE,
        // ```
        // The struct generated and all fields are `pub`.
        extra_nonce: String,
        prev_block: String,
        reserve_size: u64,
        wallet_address: String,
    },

    // The base response type.
    //
    // This is the same as the request base type,
    // it must be a type found in [`crate::base`].
    //
    // If there are any additional attributes (`/// docs` or `#[derive]`s)
    // for the struct, they go here, e.g.:
    // #[derive(Copy)]
    ResponseBase {
        // This is using `crate::base::ResponseBase`,
        // so the type we generate will contain this field:
        // ```
        // base: crate::base::ResponseBase,
        // ```
        //
        // This is flattened with serde and epee, so during
        // (de)serialization, it will act as if there are 2 extra fields here:
        // ```
        // status: crate::Status,
        // untrusted: bool,
        // ```
        blockhashing_blob: String,
        blocktemplate_blob: String,
        difficulty_top64: u64,
        difficulty: u64,
        expected_reward: u64,
        height: u64,
        next_seed_hash: String,
        prev_hash: String,
        reserved_offset: u64,
        seed_hash: String,
        seed_height: u64,
        wide_difficulty: String,
    }
}

define_request_and_response! {
    get_block_count,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 919..=933,
    GetBlockCount,

    // There is no request type specified,
    // this will cause the macro to generate a
    // type alias to `()` instead of a `struct`.
    Request {},

    ResponseBase {
        count: u64,
    }
}

define_request_and_response! {
    on_get_block_hash,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 935..=939,
    OnGetBlockHash,
    #[derive(Copy)]
    /// ```rust
    /// use serde_json::*;
    /// use cuprate_rpc_types::json::*;
    ///
    /// let x = OnGetBlockHashRequest { block_height: [3] };
    /// let x = to_string(&x).unwrap();
    /// assert_eq!(x, "[3]");
    /// ```
    #[cfg_attr(feature = "serde", serde(transparent))]
    #[repr(transparent)]
    Request {
        // This is `std::vector<u64>` in `monerod` but
        // it must be a 1 length array or else it will error.
        block_height: [u64; 1],
    },
    /// ```rust
    /// use serde_json::*;
    /// use cuprate_rpc_types::json::*;
    ///
    /// let x = OnGetBlockHashResponse { block_hash: String::from("asdf") };
    /// let x = to_string(&x).unwrap();
    /// assert_eq!(x, "\"asdf\"");
    /// ```
    #[cfg_attr(feature = "serde", serde(transparent))]
    #[repr(transparent)]
    Response {
        block_hash: String,
    }
}

define_request_and_response! {
    submit_block,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1114..=1128,
    SubmitBlock,
    /// ```rust
    /// use serde_json::*;
    /// use cuprate_rpc_types::json::*;
    ///
    /// let x = SubmitBlockRequest { block_id: String::from("asdf") };
    /// let x = to_string(&x).unwrap();
    /// assert_eq!(x, "\"asdf\"");
    /// ```
    #[cfg_attr(feature = "serde", serde(transparent))]
    #[repr(transparent)]
    Request {
        block_id: String,
    },
    /// ```rust
    /// use serde_json::*;
    /// use cuprate_rpc_types::json::*;
    ///
    /// let x = SubmitBlockResponse { status: String::from("asdf") };
    /// let x = to_string(&x).unwrap();
    /// assert_eq!(x, "\"asdf\"");
    /// ```
    #[cfg_attr(feature = "serde", serde(transparent))]
    #[repr(transparent)]
    Response {
        status: String,
    }
}

define_request_and_response! {
    generateblocks,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1130..=1161,
    GenerateBlocks,
    Request {
        amount_of_blocks: u64,
        prev_block: String,
        starting_nonce: u32,
        wallet_address: String,
    },
    ResponseBase {
        blocks: Vec<String>,
        height: u64,
    }
}

define_request_and_response! {
    get_last_block_header,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1214..=1238,
    GetLastBlockHeader,
    Request {
        #[cfg_attr(feature = "serde", serde(default = "default_bool"))]
        fill_pow_hash: bool = default_bool(),
    },
    AccessResponseBase {
        block_header: BlockHeader,
    }
}

define_request_and_response! {
    get_block_header_by_hash,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1240..=1269,
    GetBlockHeaderByHash,
    Request {
        hash: String,
        hashes: Vec<String>,
        #[cfg_attr(feature = "serde", serde(default = "default_bool"))]
        fill_pow_hash: bool = default_bool(),
    },
    AccessResponseBase {
        block_header: BlockHeader,
        block_headers: Vec<BlockHeader>,
    }
}

define_request_and_response! {
    get_block_header_by_height,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1271..=1296,
    GetBlockHeaderByHeight,
    Request {
        height: u64,
        #[cfg_attr(feature = "serde", serde(default = "default_bool"))]
        fill_pow_hash: bool = default_bool(),
    },
    AccessResponseBase {
        block_header: BlockHeader,
    }
}

define_request_and_response! {
    get_block_headers_range,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1756..=1783,
    GetBlockHeadersRange,
    Request {
        start_height: u64,
        end_height: u64,
        #[cfg_attr(feature = "serde", serde(default = "default_bool"))]
        fill_pow_hash: bool = default_bool(),
    },
    AccessResponseBase {
        headers: Vec<BlockHeader>,
    }
}

define_request_and_response! {
    get_block,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1298..=1313,
    GetBlock,
    Request {
        // `monerod` has both `hash` and `height` fields.
        // In the RPC handler, if `hash.is_empty()`, it will use it, else, it uses `height`.
        // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L2674>
        #[cfg_attr(feature = "serde", serde(default = "default_string"))]
        hash: String = default_string(),
        #[cfg_attr(feature = "serde", serde(default = "default_height"))]
        height: u64 = default_height(),
        #[cfg_attr(feature = "serde", serde(default = "default_bool"))]
        fill_pow_hash: bool = default_bool(),
    },
    AccessResponseBase {
        blob: String,
        block_header: BlockHeader,
        json: String, // TODO: this should be defined in a struct, it has many fields.
        miner_tx_hash: String,
        tx_hashes: Vec<String>,
    }
}

define_request_and_response! {
    get_connections,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1734..=1754,
    GetConnections,
    Request {},
    ResponseBase {
        // TODO: This is a `std::list` in `monerod` because...?
        connections: Vec<ConnectionInfo>,
    }
}

define_request_and_response! {
    get_info,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 693..=789,
    GetInfo,
    Request {},
    AccessResponseBase {
        adjusted_time: u64,
        alt_blocks_count: u64,
        block_size_limit: u64,
        block_size_median: u64,
        block_weight_limit: u64,
        block_weight_median: u64,
        bootstrap_daemon_address: String,
        busy_syncing: bool,
        cumulative_difficulty_top64: u64,
        cumulative_difficulty: u64,
        database_size: u64,
        difficulty_top64: u64,
        difficulty: u64,
        free_space: u64,
        grey_peerlist_size: u64,
        height: u64,
        height_without_bootstrap: u64,
        incoming_connections_count: u64,
        mainnet: bool,
        nettype: String,
        offline: bool,
        outgoing_connections_count: u64,
        restricted: bool,
        rpc_connections_count: u64,
        stagenet: bool,
        start_time: u64,
        synchronized: bool,
        target_height: u64,
        target: u64,
        testnet: bool,
        top_block_hash: String,
        tx_count: u64,
        tx_pool_size: u64,
        update_available: bool,
        version: String,
        was_bootstrap_ever_used: bool,
        white_peerlist_size: u64,
        wide_cumulative_difficulty: String,
        wide_difficulty: String,
    }
}

define_request_and_response! {
    hard_fork_info,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1958..=1995,
    HardForkInfo,
    Request {},
    AccessResponseBase {
        earliest_height: u64,
        enabled: bool,
        state: u32,
        threshold: u32,
        version: u8,
        votes: u32,
        voting: u8,
        window: u32,
    }
}

define_request_and_response! {
    set_bans,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2032..=2067,
    SetBans,
    Request {
        bans: Vec<SetBan>,
    },
    ResponseBase {}
}

define_request_and_response! {
    get_bans,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2032..=2067,
    GetBans,
    Request {},
    ResponseBase {
        bans: Vec<GetBan>,
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

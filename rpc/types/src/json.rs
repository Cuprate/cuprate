//! JSON types from the [`/json_rpc`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#json-rpc-methods) endpoint.
//!
//! All types are originally defined in [`rpc/core_rpc_server_commands_defs.h`](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h).

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    base::{AccessResponseBase, ResponseBase},
    defaults::{default_false, default_height, default_string, default_vec, default_zero},
    free::{is_one, is_zero},
    macros::define_request_and_response,
    misc::{
        AuxPow, BlockHeader, ChainInfo, ConnectionInfo, Distribution, GetBan, HardforkEntry,
        HistogramEntry, OutputDistributionData, SetBan, Span, Status, SyncInfoPeer, TxBacklogEntry,
    },
};

//---------------------------------------------------------------------------------------------------- Definitions
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

    // The request type.
    //
    // If `Request {/* fields */}` is provided, a struct is generate as-is.
    //
    // If `Request {}` is specified here, it will create a `pub type YOUR_REQUEST_TYPE = ()`
    // instead of a `struct`, see below in other macro definitions for an example.
    //
    // If there are any additional attributes (`/// docs` or `#[derive]`s)
    // for the struct, they go here, e.g.:
    // #[derive(Copy)]
    Request {
        // Within the `{}` is an infinite matching pattern of:
        // ```
        // $ATTRIBUTES
        // $FIELD_NAME: $FIELD_TYPE,
        // ```
        // The struct generated and all fields are `pub`.

        // This optional expression can be placed after
        // a `field: field_type`. this indicates to the
        // macro to (de)serialize this field using this
        // default expression if it doesn't exist in epee.
        //
        // See `cuprate_epee_encoding::epee_object` for info.
        //
        // The default function must be specified twice:
        //
        // 1. As an expression
        // 2. As a string literal
        //
        // For example: `extra_nonce: String /* = default_string(), "default_string" */,`
        //
        // This is a HACK since `serde`'s default attribute only takes in
        // string literals and macros (stringify) within attributes do not work.
        extra_nonce: String /* = default_expression, "default_literal" */,

        // Another optional expression:
        // This indicates to the macro to (de)serialize
        // this field as another type in epee.
        //
        // See `cuprate_epee_encoding::epee_object` for info.
        prev_block: String /* as Type */,

        // Regular fields.
        reserve_size: u64,
        wallet_address: String,
    },

    // The response type.
    //
    // If `Response {/* fields */}` is used,
    // this will generate a struct as-is.
    //
    // If a type found in [`crate::base`] is used,
    // It acts as a "base" that gets flattened into
    // the actual request type.
    //
    // "Flatten" means the field(s) of a struct gets inlined
    // directly into the struct during (de)serialization, see:
    // <https://serde.rs/field-attrs.html#flatten>.
    ResponseBase {
        // This is using [`crate::base::ResponseBase`],
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

    // There are no request fields specified,
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
    #[derive(Copy)]
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
    /// let x = SubmitBlockRequest { block_blob: ["a".into()] };
    /// let x = to_string(&x).unwrap();
    /// assert_eq!(x, r#"["a"]"#);
    /// ```
    #[cfg_attr(feature = "serde", serde(transparent))]
    #[repr(transparent)]
    Request {
        // This is `std::vector<std::string>` in `monerod` but
        // it must be a 1 length array or else it will error.
        block_blob: [String; 1],
    },
    ResponseBase {
        block_id: String,
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
    #[derive(Copy)]
    Request {
        fill_pow_hash: bool = default_false(), "default_false",
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
        fill_pow_hash: bool = default_false(), "default_false",
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
    #[derive(Copy)]
    Request {
        height: u64,
        fill_pow_hash: bool = default_false(), "default_false",
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
    #[derive(Copy)]
    Request {
        start_height: u64,
        end_height: u64,
        fill_pow_hash: bool = default_false(), "default_false",
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
        hash: String = default_string(), "default_string",
        height: u64 = default_height(), "default_height",
        fill_pow_hash: bool = default_false(), "default_false",
    },
    AccessResponseBase {
        blob: String,
        block_header: BlockHeader,
        json: String, // FIXME: this should be defined in a struct, it has many fields.
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
        // FIXME: This is a `std::list` in `monerod` because...?
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
    core_rpc_server_commands_defs.h => 1997..=2030,
    GetBans,
    Request {},
    ResponseBase {
        bans: Vec<GetBan>,
    }
}

define_request_and_response! {
    banned,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2069..=2094,
    Banned,
    #[cfg_attr(feature = "serde", serde(transparent))]
    #[repr(transparent)]
    Request {
        address: String,
    },
    Response {
        banned: bool,
        seconds: u32,
        status: Status,
    }
}

define_request_and_response! {
    flush_txpool,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2096..=2116,
    FlushTransactionPool,
    Request {
        txids: Vec<String> = default_vec::<String>(), "default_vec",
    },
    #[cfg_attr(feature = "serde", serde(transparent))]
    #[repr(transparent)]
    Response {
        status: Status,
    }
}

define_request_and_response! {
    get_output_histogram,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2118..=2168,
    GetOutputHistogram,
    Request {
        amounts: Vec<u64>,
        min_count: u64,
        max_count: u64,
        unlocked: bool,
        recent_cutoff: u64,
    },
    AccessResponseBase {
        histogram: Vec<HistogramEntry>,
    }
}

define_request_and_response! {
    get_coinbase_tx_sum,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2213..=2248,
    GetCoinbaseTxSum,
    Request {
        height: u64,
        count: u64,
    },
    AccessResponseBase {
        emission_amount: u64,
        emission_amount_top64: u64,
        fee_amount: u64,
        fee_amount_top64: u64,
        wide_emission_amount: String,
        wide_fee_amount: String,
    }
}

define_request_and_response! {
    get_version,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2170..=2211,
    GetVersion,
    Request {},
    ResponseBase {
        version: u32,
        release: bool,
        #[serde(skip_serializing_if = "is_zero")]
        current_height: u64 = default_zero::<u64>(), "default_zero",
        #[serde(skip_serializing_if = "is_zero")]
        target_height: u64 = default_zero::<u64>(), "default_zero",
        #[serde(skip_serializing_if = "Vec::is_empty")]
        hard_forks: Vec<HardforkEntry> = default_vec(), "default_vec",
    }
}

define_request_and_response! {
    get_fee_estimate,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2250..=2277,
    GetFeeEstimate,
    Request {},
    AccessResponseBase {
        fee: u64,
        fees: Vec<u64>,
        #[serde(skip_serializing_if = "is_one")]
        quantization_mask: u64,
    }
}

define_request_and_response! {
    get_alternate_chains,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2279..=2310,
    GetAlternateChains,
    Request {},
    ResponseBase {
        chains: Vec<ChainInfo>,
    }
}

define_request_and_response! {
    relay_tx,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2361..=2381,
    RelayTx,
    Request {
        txids: Vec<String>,
    },
    #[cfg_attr(feature = "serde", serde(transparent))]
    #[repr(transparent)]
    Response {
        status: Status,
    }
}

define_request_and_response! {
    sync_info,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2383..=2443,
    SyncInfo,
    Request {},
    AccessResponseBase {
        height: u64,
        next_needed_pruning_seed: u32,
        overview: String,
        // FIXME: This is a `std::list` in `monerod` because...?
        peers: Vec<SyncInfoPeer>,
        // FIXME: This is a `std::list` in `monerod` because...?
        spans: Vec<Span>,
        target_height: u64,
    }
}

define_request_and_response! {
    get_txpool_backlog,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1637..=1664,
    GetTransactionPoolBacklog,
    Request {},
    ResponseBase {
        // TODO: this is a [`BinaryString`].
        backlog: Vec<TxBacklogEntry>,
    }
}

define_request_and_response! {
    get_output_distribution,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2445..=2520,
    /// This type is also used in the (undocumented)
    /// [`/get_output_distribution.bin`](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.h#L138)
    /// binary endpoint.
    GetOutputDistribution,
    Request {
        amounts: Vec<u64>,
        binary: bool,
        compress: bool,
        cumulative: bool,
        from_height: u64,
        to_height: u64,
    },
    AccessResponseBase {
        distributions: Vec<Distribution>,
    }
}

define_request_and_response! {
    get_miner_data,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 996..=1044,
    GetMinerData,
    Request {},
    ResponseBase {
        major_version: u8,
        height: u64,
        prev_id: String,
        seed_hash: String,
        difficulty: String,
        median_weight: u64,
        already_generated_coins: u64,
    }
}

define_request_and_response! {
    prune_blockchain,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2747..=2772,
    PruneBlockchain,
    #[derive(Copy)]
    Request {
        check: bool = default_false(), "default_false",
    },
    ResponseBase {
        pruned: bool,
        pruning_seed: u32,
    }
}

define_request_and_response! {
    calc_pow,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1046..=1066,
    CalcPow,
    Request {
        major_version: u8,
        height: u64,
        block_blob: String,
        seed_hash: String,
    },
    #[cfg_attr(feature = "serde", serde(transparent))]
    #[repr(transparent)]
    Response {
        pow_hash: String,
    }
}

define_request_and_response! {
    flush_cache,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2774..=2796,
    FlushCache,
    #[derive(Copy)]
    Request {
        bad_txs: bool = default_false(), "default_false",
        bad_blocks: bool = default_false(), "default_false",
    },
    ResponseBase {}
}

define_request_and_response! {
    add_aux_pow,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1068..=1112,
    AddAuxPow,
    Request {
        blocktemplate_blob: String,
        aux_pow: Vec<AuxPow>,
    },
    ResponseBase {
      blocktemplate_blob: String,
      blockhashing_blob: String,
      merkle_root: String,
      merkle_tree_depth: u64,
      aux_pow: Vec<AuxPow>,
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

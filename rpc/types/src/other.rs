//! JSON types from the [`other`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#other-daemon-rpc-calls) endpoints.
//!
//! All types are originally defined in [`rpc/core_rpc_server_commands_defs.h`](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h).

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    base::{AccessResponseBase, ResponseBase},
    defaults::{default_false, default_string, default_true},
    macros::define_request_and_response,
    misc::{
        GetOutputsOut, OutKey, Peer, PublicNode, SpentKeyImageInfo, Status, TxEntry, TxInfo,
        TxpoolStats,
    },
};

//---------------------------------------------------------------------------------------------------- TODO
define_request_and_response! {
    get_height,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 138..=160,
    GetHeight,
    Request {},
    ResponseBase {
        hash: String,
        height: u64,
    }
}

define_request_and_response! {
    get_transactions,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 370..=451,
    GetTransactions,
    Request {
        txs_hashes: Vec<String>,
        // FIXME: this is documented as optional but it isn't serialized as an optional
        // but it is set _somewhere_ to false in `monerod`
        // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L382>
        decode_as_json: bool = default_false(), "default_false",
        prune: bool = default_false(), "default_false",
        split: bool = default_false(), "default_false",
    },
    AccessResponseBase {
        txs_as_hex: Vec<String>,
        txs_as_json: Vec<String>,
        missed_tx: Vec<String>,
        txs: Vec<TxEntry>,
    }
}

define_request_and_response! {
    get_alt_blocks_hashes,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 288..=308,
    GetAltBlocksHashes,
    Request {},
    AccessResponseBase {
        blks_hashes: Vec<String>,
    }
}

define_request_and_response! {
    is_key_image_spent,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 454..=484,
    IsKeyImageSpent,
    Request {
        key_images: Vec<String>,
    },
    AccessResponseBase {
        spent_status: Vec<u8>, // TODO: should be `KeyImageSpentStatus`.
    }
}

define_request_and_response! {
    send_raw_transaction,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 370..=451,
    SendRawTransaction,
    Request {
        tx_as_hex: String,
        do_not_relay: bool = default_false(), "default_false",
        do_sanity_checks: bool = default_true(), "default_true",
    },
    AccessResponseBase {
        double_spend: bool,
        fee_too_low: bool,
        invalid_input: bool,
        invalid_output: bool,
        low_mixin: bool,
        nonzero_unlock_time: bool,
        not_relayed: bool,
        overspend: bool,
        reason: String,
        sanity_check_failed: bool,
        too_big: bool,
        too_few_outputs: bool,
        tx_extra_too_big: bool,
    }
}

define_request_and_response! {
    start_mining,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 665..=691,
    StartMining,
    Request {
        miner_address: String,
        threads_count: u64,
        do_background_mining: bool,
        ignore_battery: bool,
    },
    ResponseBase {}
}

define_request_and_response! {
    stop_mining,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 825..=843,
    StopMining,
    Request {},
    ResponseBase {}
}

define_request_and_response! {
    mining_status,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 846..=895,
    MiningStatus,
    Request {},
    ResponseBase {
        active: bool,
        address: String,
        bg_idle_threshold: u8,
        bg_ignore_battery: bool,
        bg_min_idle_seconds: u8,
        bg_target: u8,
        block_reward: u64,
        block_target: u32,
        difficulty: u64,
        difficulty_top64: u64,
        is_background_mining_enabled: bool,
        pow_algorithm: String,
        speed: u64,
        threads_count: u32,
        wide_difficulty: String,
    }
}

define_request_and_response! {
    save_bc,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 898..=916,
    SaveBc,
    Request {},
    ResponseBase {}
}

define_request_and_response! {
    get_peer_list,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1369..=1417,
    GetPeerList,
    Request {
        public_only: bool = default_true(), "default_true",
        include_blocked: bool = default_false(), "default_false",
    },
    ResponseBase {
        white_list: Vec<Peer>,
        gray_list: Vec<Peer>,
    }
}

define_request_and_response! {
    set_log_hash_rate,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1450..=1470,
    SetLogHashRate,
    #[derive(Copy)]
    Request {
        visible: bool,
    },
    ResponseBase {}
}

define_request_and_response! {
    set_log_level,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1450..=1470,
    SetLogLevel,
    #[derive(Copy)]
    Request {
        level: u8,
    },
    ResponseBase {}
}

define_request_and_response! {
    set_log_categories,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1494..=1517,
    SetLogCategories,
    Request {
        categories: String = default_string(), "default_string",
    },
    ResponseBase {
        categories: String,
    }
}

define_request_and_response! {
    set_bootstrap_daemon,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1785..=1812,
    SetBootstrapDaemon,
    Request {
        address: String,
        username: String,
        password: String,
        proxy: String,
    },
    #[derive(Copy)]
    Response {
        status: Status,
    }
}

define_request_and_response! {
    get_transaction_pool,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1569..=1591,
    GetTransactionPool,
    Request {},
    AccessResponseBase {
        transactions: Vec<TxInfo>,
        spent_key_images: Vec<SpentKeyImageInfo>,
    }
}

define_request_and_response! {
    get_transaction_pool_stats,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1712..=1732,
    GetTransactionPoolStats,
    Request {},
    AccessResponseBase {
        pool_stats: TxpoolStats,
    }
}

define_request_and_response! {
    stop_daemon,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1814..=1831,
    StopDaemon,
    Request {},
    ResponseBase {
        status: Status,
    }
}

define_request_and_response! {
    get_limit,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1852..=1874,
    GetLimit,
    Request {},
    ResponseBase {
        limit_down: u64,
        limit_up: u64,
    }
}

define_request_and_response! {
    set_limit,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1876..=1903,
    SetLimit,
    Request {
        limit_down: i64,
        limit_up: i64,
    },
    ResponseBase {
        limit_down: i64,
        limit_up: i64,
    }
}

define_request_and_response! {
    out_peers,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1876..=1903,
    OutPeers,
    Request {
        set: bool = default_true(), "default_true",
        out_peers: u32,
    },
    ResponseBase {
        out_peers: u32,
    }
}

define_request_and_response! {
    get_net_stats,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 793..=822,
    GetNetStats,
    Request {},
    ResponseBase {
        start_time: u64,
        total_packets_in: u64,
        total_bytes_in: u64,
        total_packets_out: u64,
        total_bytes_out: u64,
    }
}

define_request_and_response! {
    get_outs,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 567..=609,
    GetOuts,
    Request {
        outputs: Vec<GetOutputsOut>,
        get_txid: bool,
    },
    ResponseBase {
        outs: Vec<OutKey>,
    }
}

define_request_and_response! {
    update,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2324..=2359,
    Update,
    Request {
        command: String,
        path: String = default_string(), "default_string",
    },
    ResponseBase {
        auto_uri: String,
        hash: String,
        path: String,
        update: bool,
        user_uri: String,
        version: String,
    }
}

define_request_and_response! {
    pop_blocks,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2722..=2745,
    PopBlocks,
    Request {
        nblocks: u64,
    },
    ResponseBase {
        height: u64,
    }
}

define_request_and_response! {
    UNDOCUMENTED_ENDPOINT,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2798..=2823,
    GetTxIdsLoose,
    Request {
        txid_template: String,
        num_matching_bits: u32,
    },
    ResponseBase {
        txids: Vec<String>,
    }
}

define_request_and_response! {
    UNDOCUMENTED_ENDPOINT,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1615..=1635,
    GetTransactionPoolHashes,
    Request {},
    ResponseBase {
        tx_hashes: Vec<String>,
    }
}

define_request_and_response! {
    UNDOCUMENTED_ENDPOINT,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1419..=1448,
    GetPublicNodes,
    Request {
        gray: bool = default_false(), "default_false",
        white: bool = default_true(), "default_true",
        include_blocked: bool = default_false(), "default_false",
    },
    ResponseBase {
        gray: Vec<PublicNode>,
        white: Vec<PublicNode>,
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

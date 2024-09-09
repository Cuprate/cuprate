//! JSON types from the [`other`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#other-daemon-rpc-calls) endpoints.
//!
//! All types are originally defined in [`rpc/core_rpc_server_commands_defs.h`](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h).

//---------------------------------------------------------------------------------------------------- Import
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    base::{AccessResponseBase, ResponseBase},
    defaults::{default_false, default_string, default_true, default_vec, default_zero},
    macros::define_request_and_response,
    misc::{
        GetOutputsOut, OutKey, Peer, PublicNode, SpentKeyImageInfo, Status, TxEntry, TxInfo,
        TxpoolStats,
    },
    RpcCallValue,
};

//---------------------------------------------------------------------------------------------------- Macro
/// Adds a (de)serialization doc-test to a type in `other.rs`.
///
/// It expects a const string from `cuprate_test_utils::rpc::data`
/// and the expected value it should (de)serialize into/from.
///
/// It tests that the provided const JSON string can properly
/// (de)serialize into the expected value.
///
/// See below for example usage. This macro is only used in this file.
macro_rules! serde_doc_test {
    // This branch _only_ tests that the type can be deserialize
    // from the string, not that any value is correct.
    //
    // Practically, this is used for structs that have
    // many values that are complicated to test, e.g. `GET_TRANSACTIONS_RESPONSE`.
    //
    // HACK:
    // The type itself doesn't need to be specified because it happens
    // to just be the `CamelCase` version of the provided const.
    (
        // `const` string from `cuprate_test_utils::rpc::data`.
        $cuprate_test_utils_rpc_const:ident
    ) => {
        paste::paste! {
            concat!(
                "```rust\n",
                "use cuprate_test_utils::rpc::data::other::*;\n",
                "use cuprate_rpc_types::{misc::*, base::*, other::*};\n",
                "use serde_json::{Value, from_str, from_value};\n",
                "\n",
                "let string = from_str::<",
                stringify!([<$cuprate_test_utils_rpc_const:camel>]),
                ">(",
                stringify!($cuprate_test_utils_rpc_const),
                ").unwrap();\n",
                "```\n",
            )
        }
    };

    // This branch tests that the type can be deserialize
    // from the string AND that values are correct.
    (
        // `const` string from `cuprate_test_utils::rpc::data`
        //  v
        $cuprate_test_utils_rpc_const:ident => $expected:expr
        //                                     ^
        //                     Expected value as an expression
    ) => {
        paste::paste! {
            concat!(
                "```rust\n",
                "use cuprate_test_utils::rpc::data::other::*;\n",
                "use cuprate_rpc_types::{misc::*, base::*, other::*};\n",
                "use serde_json::{Value, from_str, from_value};\n",
                "\n",
                "// The expected data.\n",
                "let expected = ",
                stringify!($expected),
                ";\n",
                "\n",
                "let string = from_str::<",
                stringify!([<$cuprate_test_utils_rpc_const:camel>]),
                ">(",
                stringify!($cuprate_test_utils_rpc_const),
                ").unwrap();\n",
                "\n",
                "assert_eq!(string, expected);\n",
                "```\n",
            )
        }
    };
}

//---------------------------------------------------------------------------------------------------- Definitions
define_request_and_response! {
    get_height,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 138..=160,
    GetHeight (empty),
    Request {},

    #[doc = serde_doc_test!(
        GET_HEIGHT_RESPONSE => GetHeightResponse {
            base: ResponseBase::ok(),
            hash: "68bb1a1cff8e2a44c3221e8e1aff80bc6ca45d06fa8eff4d2a3a7ac31d4efe3f".into(),
            height: 3195160,
        }
    )]
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

    #[doc = serde_doc_test!(
        GET_TRANSACTIONS_REQUEST => GetTransactionsRequest {
            txs_hashes: vec!["d6e48158472848e6687173a91ae6eebfa3e1d778e65252ee99d7515d63090408".into()],
            decode_as_json: false,
            prune: false,
            split: false,
        }
    )]
    Request {
        txs_hashes: Vec<String>,
        // FIXME: this is documented as optional but it isn't serialized as an optional
        // but it is set _somewhere_ to false in `monerod`
        // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L382>
        decode_as_json: bool = default_false(), "default_false",
        prune: bool = default_false(), "default_false",
        split: bool = default_false(), "default_false",
    },

    #[doc = serde_doc_test!(GET_TRANSACTIONS_RESPONSE)]
    AccessResponseBase {
        txs_as_hex: Vec<String> = default_vec::<String>(), "default_vec",
        txs_as_json: Vec<String> = default_vec::<String>(), "default_vec",
        missed_tx: Vec<String> = default_vec::<String>(), "default_vec",
        txs: Vec<TxEntry> = default_vec::<TxEntry>(), "default_vec",
    }
}

define_request_and_response! {
    get_alt_blocks_hashes,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 288..=308,
    GetAltBlocksHashes (empty),
    Request {},

    #[doc = serde_doc_test!(
        GET_ALT_BLOCKS_HASHES_RESPONSE => GetAltBlocksHashesResponse {
            base: AccessResponseBase::ok(),
            blks_hashes: vec!["8ee10db35b1baf943f201b303890a29e7d45437bd76c2bd4df0d2f2ee34be109".into()],
        }
    )]
    AccessResponseBase {
        blks_hashes: Vec<String>,
    }
}

define_request_and_response! {
    is_key_image_spent,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 454..=484,

    IsKeyImageSpent,

    #[doc = serde_doc_test!(
        IS_KEY_IMAGE_SPENT_REQUEST => IsKeyImageSpentRequest {
            key_images: vec![
                "8d1bd8181bf7d857bdb281e0153d84cd55a3fcaa57c3e570f4a49f935850b5e3".into(),
                "7319134bfc50668251f5b899c66b005805ee255c136f0e1cecbb0f3a912e09d4".into()
            ]
        }
    )]
    Request {
        key_images: Vec<String>,
    },

    #[doc = serde_doc_test!(
        IS_KEY_IMAGE_SPENT_RESPONSE => IsKeyImageSpentResponse {
            base: AccessResponseBase::ok(),
            spent_status: vec![1, 1],
        }
    )]
    AccessResponseBase {
        /// FIXME: These are [`KeyImageSpentStatus`](crate::misc::KeyImageSpentStatus) in [`u8`] form.
        spent_status: Vec<u8>,
    }
}

define_request_and_response! {
    send_raw_transaction,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 370..=451,

    SendRawTransaction,

    #[doc = serde_doc_test!(
        SEND_RAW_TRANSACTION_REQUEST => SendRawTransactionRequest {
            tx_as_hex: "dc16fa8eaffe1484ca9014ea050e13131d3acf23b419f33bb4cc0b32b6c49308".into(),
            do_not_relay: false,
            do_sanity_checks: true,
        }
    )]
    Request {
        tx_as_hex: String,
        do_not_relay: bool = default_false(), "default_false",
        do_sanity_checks: bool = default_true(), "default_true",
    },

    #[doc = serde_doc_test!(
        SEND_RAW_TRANSACTION_RESPONSE => SendRawTransactionResponse {
            base: AccessResponseBase {
                response_base: ResponseBase {
                    status: Status::Other("Failed".into()),
                    untrusted: false,
                },
                credits: 0,
                top_hash: "".into(),
            },
            double_spend: false,
            fee_too_low: false,
            invalid_input: false,
            invalid_output: false,
            low_mixin: false,
            not_relayed: false,
            overspend: false,
            reason: "".into(),
            sanity_check_failed: false,
            too_big: false,
            too_few_outputs: false,
            tx_extra_too_big: false,
            nonzero_unlock_time: false,
        }
    )]
    AccessResponseBase {
        double_spend: bool,
        fee_too_low: bool,
        invalid_input: bool,
        invalid_output: bool,
        low_mixin: bool,
        nonzero_unlock_time: bool = default_false(), "default_false",
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

    StartMining (restricted),

    #[doc = serde_doc_test!(
        START_MINING_REQUEST => StartMiningRequest {
            do_background_mining: false,
            ignore_battery: true,
            miner_address: "47xu3gQpF569au9C2ajo5SSMrWji6xnoE5vhr94EzFRaKAGw6hEGFXYAwVADKuRpzsjiU1PtmaVgcjUJF89ghGPhUXkndHc".into(),
            threads_count: 1
        }
    )]
    Request {
        miner_address: String,
        threads_count: u64,
        do_background_mining: bool,
        ignore_battery: bool,
    },

    #[doc = serde_doc_test!(
        START_MINING_RESPONSE => StartMiningResponse {
            base: ResponseBase::ok(),
        }
    )]
    ResponseBase {}
}

define_request_and_response! {
    stop_mining,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 825..=843,
    StopMining (restricted, empty),
    Request {},

    #[doc = serde_doc_test!(
        STOP_MINING_RESPONSE => StopMiningResponse {
            base: ResponseBase::ok(),
        }
    )]
    ResponseBase {}
}

define_request_and_response! {
    mining_status,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 846..=895,
    MiningStatus (restricted),
    Request {},

    #[doc = serde_doc_test!(
        MINING_STATUS_RESPONSE => MiningStatusResponse {
            base: ResponseBase::ok(),
            active: false,
            address: "".into(),
            bg_idle_threshold: 0,
            bg_ignore_battery: false,
            bg_min_idle_seconds: 0,
            bg_target: 0,
            block_reward: 0,
            block_target: 120,
            difficulty: 292022797663,
            difficulty_top64: 0,
            is_background_mining_enabled: false,
            pow_algorithm: "RandomX".into(),
            speed: 0,
            threads_count: 0,
            wide_difficulty: "0x43fdea455f".into(),
        }
    )]
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
    SaveBc (restricted),
    Request {},

    #[doc = serde_doc_test!(
        SAVE_BC_RESPONSE => SaveBcResponse {
            base: ResponseBase::ok(),
        }
    )]
    ResponseBase {}
}

define_request_and_response! {
    get_peer_list,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1369..=1417,

    GetPeerList (restricted),

    #[doc = serde_doc_test!(
        GET_PEER_LIST_REQUEST => GetPeerListRequest {
            public_only: true,
            include_blocked: false,
        }
    )]
    Request {
        public_only: bool = default_true(), "default_true",
        include_blocked: bool = default_false(), "default_false",
    },

    #[doc = serde_doc_test!(
        GET_PEER_LIST_RESPONSE => GetPeerListResponse {
            base: ResponseBase::ok(),
            gray_list: vec![
                Peer {
                    host: "161.97.193.0".into(),
                    id: 18269586253849566614,
                    ip: 12673441,
                    last_seen: 0,
                    port: 18080,
                    rpc_port: 0,
                    rpc_credits_per_hash: 0,
                    pruning_seed: 0,
                },
                Peer {
                    host: "193.142.4.2".into(),
                    id: 10865563782170056467,
                    ip: 33853121,
                    last_seen: 0,
                    port: 18085,
                    pruning_seed: 387,
                    rpc_port: 19085,
                    rpc_credits_per_hash: 0,
                }
            ],
            white_list: vec![
                Peer {
                    host: "78.27.98.0".into(),
                    id: 11368279936682035606,
                    ip: 6429518,
                    last_seen: 1721246387,
                    port: 18080,
                    pruning_seed: 384,
                    rpc_port: 0,
                    rpc_credits_per_hash: 0,
                },
                Peer {
                    host: "67.4.163.2".into(),
                    id: 16545113262826842499,
                    ip: 44237891,
                    last_seen: 1721246387,
                    port: 18080,
                    rpc_port: 0,
                    rpc_credits_per_hash: 0,
                    pruning_seed: 0,
                },
                Peer {
                    host: "70.52.75.3".into(),
                    id: 3863337548778177169,
                    ip: 55260230,
                    last_seen: 1721246387,
                    port: 18080,
                    rpc_port: 18081,
                    rpc_credits_per_hash: 0,
                    pruning_seed: 0,
                }
            ]
        }
    )]
    ResponseBase {
        white_list: Vec<Peer>,
        gray_list: Vec<Peer>,
    }
}

define_request_and_response! {
    set_log_hash_rate,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1450..=1470,

    SetLogHashRate (restricted),

    #[derive(Copy)]
    #[doc = serde_doc_test!(
        SET_LOG_HASH_RATE_REQUEST => SetLogHashRateRequest {
            visible: true,
        }
    )]
    Request {
        visible: bool = default_false(), "default_false",
    },

    #[doc = serde_doc_test!(
        SET_LOG_HASH_RATE_RESPONSE => SetLogHashRateResponse {
            base: ResponseBase::ok(),
        }
    )]
    ResponseBase {}
}

define_request_and_response! {
    set_log_level,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1450..=1470,

    SetLogLevel (restricted),

    #[derive(Copy)]
    #[doc = serde_doc_test!(
        SET_LOG_LEVEL_REQUEST => SetLogLevelRequest {
            level: 1
        }
    )]
    Request {
        level: u8,
    },

    #[doc = serde_doc_test!(
        SET_LOG_LEVEL_RESPONSE => SetLogLevelResponse {
            base: ResponseBase::ok(),
        }
    )]
    ResponseBase {}
}

define_request_and_response! {
    set_log_categories,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1494..=1517,

    SetLogCategories (restricted),

    #[doc = serde_doc_test!(
        SET_LOG_CATEGORIES_REQUEST => SetLogCategoriesRequest {
            categories: "*:INFO".into(),
        }
    )]
    Request {
        categories: String = default_string(), "default_string",
    },

    #[doc = serde_doc_test!(
        SET_LOG_CATEGORIES_RESPONSE => SetLogCategoriesResponse {
            base: ResponseBase::ok(),
            categories: "*:INFO".into(),
        }
    )]
    ResponseBase {
        categories: String,
    }
}

define_request_and_response! {
    set_bootstrap_daemon,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1785..=1812,

    SetBootstrapDaemon (restricted),

    #[doc = serde_doc_test!(
        SET_BOOTSTRAP_DAEMON_REQUEST => SetBootstrapDaemonRequest {
            address: "http://getmonero.org:18081".into(),
            username: String::new(),
            password: String::new(),
            proxy: String::new(),
        }
    )]
    Request {
        address: String,
        username: String = default_string(), "default_string",
        password: String = default_string(), "default_string",
        proxy: String = default_string(), "default_string",
    },

    #[doc = serde_doc_test!(
        SET_BOOTSTRAP_DAEMON_RESPONSE => SetBootstrapDaemonResponse {
            status: Status::Ok,
        }
    )]
    Response {
        status: Status,
    }
}

define_request_and_response! {
    get_transaction_pool,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1569..=1591,

    GetTransactionPool (empty),
    Request {},

    #[doc = serde_doc_test!(GET_TRANSACTION_POOL_RESPONSE)]
    AccessResponseBase {
        transactions: Vec<TxInfo>,
        spent_key_images: Vec<SpentKeyImageInfo>,
    }
}

define_request_and_response! {
    get_transaction_pool_stats,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1712..=1732,

    GetTransactionPoolStats (empty),
    Request {},

    #[doc = serde_doc_test!(
        GET_TRANSACTION_POOL_STATS_RESPONSE => GetTransactionPoolStatsResponse {
            base: AccessResponseBase::ok(),
            pool_stats: TxpoolStats {
                bytes_max: 11843,
                bytes_med: 2219,
                bytes_min: 1528,
                bytes_total: 144192,
                fee_total: 7018100000,
                histo: vec![
                    TxpoolHisto { bytes: 11219, txs: 4 },
                    TxpoolHisto { bytes: 9737, txs: 5 },
                    TxpoolHisto { bytes: 8757, txs: 4 },
                    TxpoolHisto { bytes: 14763, txs: 4 },
                    TxpoolHisto { bytes: 15007, txs: 6 },
                    TxpoolHisto { bytes: 15924, txs: 6 },
                    TxpoolHisto { bytes: 17869, txs: 8 },
                    TxpoolHisto { bytes: 10894, txs: 5 },
                    TxpoolHisto { bytes: 38485, txs: 10 },
                    TxpoolHisto { bytes: 1537, txs: 1 },
                ],
                histo_98pc: 186,
                num_10m: 0,
                num_double_spends: 0,
                num_failing: 0,
                num_not_relayed: 0,
                oldest: 1721261651,
                txs_total: 53
            }
        }
    )]
    AccessResponseBase {
        pool_stats: TxpoolStats,
    }
}

define_request_and_response! {
    stop_daemon,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1814..=1831,

    StopDaemon (restricted, empty),
    Request {},

    #[doc = serde_doc_test!(
        STOP_DAEMON_RESPONSE => StopDaemonResponse {
            status: Status::Ok,
        }
    )]
    Response {
        status: Status,
    }
}

define_request_and_response! {
    get_limit,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1852..=1874,

    GetLimit (empty),
    Request {},

    #[doc = serde_doc_test!(
        GET_LIMIT_RESPONSE => GetLimitResponse {
            base: ResponseBase::ok(),
            limit_down: 1280000,
            limit_up: 1280000,
        }
    )]
    ResponseBase {
        limit_down: u64,
        limit_up: u64,
    }
}

define_request_and_response! {
    set_limit,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1876..=1903,

    SetLimit (restricted),

    #[doc = serde_doc_test!(
        SET_LIMIT_REQUEST => SetLimitRequest {
            limit_down: 1024,
            limit_up: 0,
        }
    )]
    Request {
        // FIXME: These may need to be `Option<i64>`.
        limit_down: i64 = default_zero::<i64>(), "default_zero",
        limit_up: i64 = default_zero::<i64>(), "default_zero",
    },

    #[doc = serde_doc_test!(
        SET_LIMIT_RESPONSE => SetLimitResponse {
            base: ResponseBase::ok(),
            limit_down: 1024,
            limit_up: 128,
        }
    )]
    ResponseBase {
        limit_down: i64,
        limit_up: i64,
    }
}

define_request_and_response! {
    out_peers,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1876..=1903,

    OutPeers (restricted),

    #[doc = serde_doc_test!(
        OUT_PEERS_REQUEST => OutPeersRequest {
            out_peers: 3232235535,
            set: true,
        }
    )]
    Request {
        set: bool = default_true(), "default_true",
        out_peers: u32,
    },

    #[doc = serde_doc_test!(
        OUT_PEERS_RESPONSE => OutPeersResponse {
            base: ResponseBase::ok(),
            out_peers: 3232235535,
        }
    )]
    ResponseBase {
        out_peers: u32,
    }
}

define_request_and_response! {
    in_peers,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1932..=1956,
    InPeers (restricted),
    Request {
        set: bool = default_true(), "default_true",
        in_peers: u32,
    },
    ResponseBase {
        in_peers: u32,
    }
}

define_request_and_response! {
    get_net_stats,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 793..=822,

    GetNetStats (restricted, empty),
    Request {},

    #[doc = serde_doc_test!(
        GET_NET_STATS_RESPONSE => GetNetStatsResponse {
            base: ResponseBase::ok(),
            start_time: 1721251858,
            total_bytes_in: 16283817214,
            total_bytes_out: 34225244079,
            total_packets_in: 5981922,
            total_packets_out: 3627107,
        }
    )]
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
    #[doc = serde_doc_test!(
        GET_OUTS_REQUEST => GetOutsRequest {
            outputs: vec![
                GetOutputsOut { amount: 1, index: 0 },
                GetOutputsOut { amount: 1, index: 1 },
            ],
            get_txid: true
        }
    )]
    Request {
        outputs: Vec<GetOutputsOut>,
        get_txid: bool,
    },

    #[doc = serde_doc_test!(
        GET_OUTS_RESPONSE => GetOutsResponse {
            base: ResponseBase::ok(),
            outs: vec![
                OutKey {
                    height: 51941,
                    key: "08980d939ec297dd597119f498ad69fed9ca55e3a68f29f2782aae887ef0cf8e".into(),
                    mask: "1738eb7a677c6149228a2beaa21bea9e3370802d72a3eec790119580e02bd522".into(),
                    txid: "9d651903b80fb70b9935b72081cd967f543662149aed3839222511acd9100601".into(),
                    unlocked: true
                },
                OutKey {
                    height: 51945,
                    key: "454fe46c405be77625fa7e3389a04d3be392346983f27603561ac3a3a74f4a75".into(),
                    mask: "1738eb7a677c6149228a2beaa21bea9e3370802d72a3eec790119580e02bd522".into(),
                    txid: "230bff732dc5f225df14fff82aadd1bf11b3fb7ad3a03413c396a617e843f7d0".into(),
                    unlocked: true
                },
            ]
        }
    )]
    ResponseBase {
        outs: Vec<OutKey>,
    }
}

define_request_and_response! {
    update,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 2324..=2359,

    Update (restricted),

    #[doc = serde_doc_test!(
        UPDATE_REQUEST => UpdateRequest {
            command: "check".into(),
            path: "".into(),
        }
    )]
    Request {
        command: String,
        path: String = default_string(), "default_string",
    },

    #[doc = serde_doc_test!(
        UPDATE_RESPONSE => UpdateResponse {
            base: ResponseBase::ok(),
            auto_uri: "".into(),
            hash: "".into(),
            path: "".into(),
            update: false,
            user_uri: "".into(),
            version: "".into(),
        }
    )]
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

    PopBlocks (restricted),

    #[doc = serde_doc_test!(
        POP_BLOCKS_REQUEST => PopBlocksRequest {
            nblocks: 6
        }
    )]
    Request {
        nblocks: u64,
    },

    #[doc = serde_doc_test!(
        POP_BLOCKS_RESPONSE => PopBlocksResponse {
            base: ResponseBase::ok(),
            height: 76482,
        }
    )]
    ResponseBase {
        height: u64,
    }
}

define_request_and_response! {
    UNDOCUMENTED_ENDPOINT,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1615..=1635,

    GetTransactionPoolHashes (empty),
    Request {},

    #[doc = serde_doc_test!(
        GET_TRANSACTION_POOL_HASHES_RESPONSE => GetTransactionPoolHashesResponse {
            base: ResponseBase::ok(),
            tx_hashes: vec![
                "aa928aed888acd6152c60194d50a4df29b0b851be6169acf11b6a8e304dd6c03".into(),
                "794345f321a98f3135151f3056c0fdf8188646a8dab27de971428acf3551dd11".into(),
                "1e9d2ae11f2168a228942077483e70940d34e8658c972bbc3e7f7693b90edf17".into(),
                "7375c928f261d00f07197775eb0bfa756e5f23319819152faa0b3c670fe54c1b".into(),
                "2e4d5f8c5a45498f37fb8b6ca4ebc1efa0c371c38c901c77e66b08c072287329".into(),
                "eee6d596cf855adfb10e1597d2018e3a61897ac467ef1d4a5406b8d20bfbd52f".into(),
                "59c574d7ba9bb4558470f74503c7518946a85ea22c60fccfbdec108ce7d8f236".into(),
                "0d57bec1e1075a9e1ac45cf3b3ced1ad95ccdf2a50ce360190111282a0178655".into(),
                "60d627b2369714a40009c07d6185ebe7fa4af324fdfa8d95a37a936eb878d062".into(),
                "661d7e728a901a8cb4cf851447d9cd5752462687ed0b776b605ba706f06bdc7d".into(),
                "b80e1f09442b00b3fffe6db5d263be6267c7586620afff8112d5a8775a6fc58e".into(),
                "974063906d1ddfa914baf85176b0f689d616d23f3d71ed4798458c8b4f9b9d8f".into(),
                "d2575ae152a180be4981a9d2fc009afcd073adaa5c6d8b022c540a62d6c905bb".into(),
                "3d78aa80ee50f506683bab9f02855eb10257a08adceda7cbfbdfc26b10f6b1bb".into(),
                "8b5bc125bdb73b708500f734501d55088c5ac381a0879e1141634eaa72b6a4da".into(),
                "11c06f4d2f00c912ca07313ed2ea5366f3cae914a762bed258731d3d9e3706df".into(),
                "b3644dc7c9a3a53465fe80ad3769e516edaaeb7835e16fdd493aac110d472ae1".into(),
                "ed2478ad793b923dbf652c8612c40799d764e5468897021234a14a37346bc6ee".into()
            ],
        }
    )]
    ResponseBase {
        tx_hashes: Vec<String>,
    }
}

define_request_and_response! {
    UNDOCUMENTED_ENDPOINT,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1419..=1448,

    GetPublicNodes (restricted),

    #[doc = serde_doc_test!(
        GET_PUBLIC_NODES_REQUEST => GetPublicNodesRequest {
            gray: false,
            white: true,
            include_blocked: false,
        }
    )]
    Request {
        gray: bool = default_false(), "default_false",
        white: bool = default_true(), "default_true",
        include_blocked: bool = default_false(), "default_false",
    },

    #[doc = serde_doc_test!(
        GET_PUBLIC_NODES_RESPONSE => GetPublicNodesResponse {
            base: ResponseBase::ok(),
            gray: vec![],
            white: vec![
                PublicNode {
                    host: "70.52.75.3".into(),
                    last_seen: 1721246387,
                    rpc_credits_per_hash: 0,
                    rpc_port: 18081,
                },
                PublicNode {
                    host: "zbjkbsxc5munw3qusl7j2hpcmikhqocdf4pqhnhtpzw5nt5jrmofptid.onion:18083".into(),
                    last_seen: 1720186288,
                    rpc_credits_per_hash: 0,
                    rpc_port: 18089,
                }
            ]
        }
    )]
    ResponseBase {
        gray: Vec<PublicNode> = default_vec::<PublicNode>(), "default_vec",
        white: Vec<PublicNode> = default_vec::<PublicNode>(), "default_vec",
    }
}

//---------------------------------------------------------------------------------------------------- Request
/// Other JSON requests.
///
/// This enum contains all [`crate::other`] requests.
///
/// See also: [`OtherResponse`].
///
/// # (De)serialization
/// The `serde` implementation will (de)serialize from
/// the inner variant itself, e.g. [`OtherRequest::SetLogLevel`]
/// has the same (de)serialization as [`SetLogLevelRequest`].
///
/// ```rust
/// use cuprate_rpc_types::other::*;
///
/// let request = OtherRequest::SetLogLevel(Default::default());
/// let json = serde_json::to_string(&request).unwrap();
/// assert_eq!(json, r#"{"level":0}"#);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[allow(missing_docs)]
pub enum OtherRequest {
    GetHeight(GetHeightRequest),
    GetTransactions(GetTransactionsRequest),
    GetAltBlocksHashes(GetAltBlocksHashesRequest),
    IsKeyImageSpent(IsKeyImageSpentRequest),
    SendRawTransaction(SendRawTransactionRequest),
    StartMining(StartMiningRequest),
    StopMining(StopMiningRequest),
    MiningStatus(MiningStatusRequest),
    SaveBc(SaveBcRequest),
    GetPeerList(GetPeerListRequest),
    SetLogHashRate(SetLogHashRateRequest),
    SetLogLevel(SetLogLevelRequest),
    SetLogCategories(SetLogCategoriesRequest),
    SetBootstrapDaemon(SetBootstrapDaemonRequest),
    GetTransactionPool(GetTransactionPoolRequest),
    GetTransactionPoolStats(GetTransactionPoolStatsRequest),
    StopDaemon(StopDaemonRequest),
    GetLimit(GetLimitRequest),
    SetLimit(SetLimitRequest),
    OutPeers(OutPeersRequest),
    InPeers(InPeersRequest),
    GetNetStats(GetNetStatsRequest),
    GetOuts(GetOutsRequest),
    Update(UpdateRequest),
    PopBlocks(PopBlocksRequest),
    GetTransactionPoolHashes(GetTransactionPoolHashesRequest),
    GetPublicNodes(GetPublicNodesRequest),
}

impl RpcCallValue for OtherRequest {
    fn is_restricted(&self) -> bool {
        match self {
            Self::GetHeight(x) => x.is_restricted(),
            Self::GetTransactions(x) => x.is_restricted(),
            Self::GetAltBlocksHashes(x) => x.is_restricted(),
            Self::IsKeyImageSpent(x) => x.is_restricted(),
            Self::SendRawTransaction(x) => x.is_restricted(),
            Self::StartMining(x) => x.is_restricted(),
            Self::StopMining(x) => x.is_restricted(),
            Self::MiningStatus(x) => x.is_restricted(),
            Self::SaveBc(x) => x.is_restricted(),
            Self::GetPeerList(x) => x.is_restricted(),
            Self::SetLogHashRate(x) => x.is_restricted(),
            Self::SetLogLevel(x) => x.is_restricted(),
            Self::SetLogCategories(x) => x.is_restricted(),
            Self::SetBootstrapDaemon(x) => x.is_restricted(),
            Self::GetTransactionPool(x) => x.is_restricted(),
            Self::GetTransactionPoolStats(x) => x.is_restricted(),
            Self::StopDaemon(x) => x.is_restricted(),
            Self::GetLimit(x) => x.is_restricted(),
            Self::SetLimit(x) => x.is_restricted(),
            Self::OutPeers(x) => x.is_restricted(),
            Self::InPeers(x) => x.is_restricted(),
            Self::GetNetStats(x) => x.is_restricted(),
            Self::GetOuts(x) => x.is_restricted(),
            Self::Update(x) => x.is_restricted(),
            Self::PopBlocks(x) => x.is_restricted(),
            Self::GetTransactionPoolHashes(x) => x.is_restricted(),
            Self::GetPublicNodes(x) => x.is_restricted(),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Self::GetHeight(x) => x.is_empty(),
            Self::GetTransactions(x) => x.is_empty(),
            Self::GetAltBlocksHashes(x) => x.is_empty(),
            Self::IsKeyImageSpent(x) => x.is_empty(),
            Self::SendRawTransaction(x) => x.is_empty(),
            Self::StartMining(x) => x.is_empty(),
            Self::StopMining(x) => x.is_empty(),
            Self::MiningStatus(x) => x.is_empty(),
            Self::SaveBc(x) => x.is_empty(),
            Self::GetPeerList(x) => x.is_empty(),
            Self::SetLogHashRate(x) => x.is_empty(),
            Self::SetLogLevel(x) => x.is_empty(),
            Self::SetLogCategories(x) => x.is_empty(),
            Self::SetBootstrapDaemon(x) => x.is_empty(),
            Self::GetTransactionPool(x) => x.is_empty(),
            Self::GetTransactionPoolStats(x) => x.is_empty(),
            Self::StopDaemon(x) => x.is_empty(),
            Self::GetLimit(x) => x.is_empty(),
            Self::SetLimit(x) => x.is_empty(),
            Self::OutPeers(x) => x.is_empty(),
            Self::InPeers(x) => x.is_empty(),
            Self::GetNetStats(x) => x.is_empty(),
            Self::GetOuts(x) => x.is_empty(),
            Self::Update(x) => x.is_empty(),
            Self::PopBlocks(x) => x.is_empty(),
            Self::GetTransactionPoolHashes(x) => x.is_empty(),
            Self::GetPublicNodes(x) => x.is_empty(),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Response
/// Other JSON responses.
///
/// This enum contains all [`crate::other`] responses.
///
/// See also: [`OtherRequest`].
///
/// # (De)serialization
/// The `serde` implementation will (de)serialize from
/// the inner variant itself, e.g. [`OtherRequest::SetBootstrapDaemon`]
/// has the same (de)serialization as [`SetBootstrapDaemonResponse`].
///
/// ```rust
/// use cuprate_rpc_types::other::*;
///
/// let response = OtherResponse::SetBootstrapDaemon(Default::default());
/// let json = serde_json::to_string(&response).unwrap();
/// assert_eq!(json, r#"{"status":"OK"}"#);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[allow(missing_docs)]
pub enum OtherResponse {
    GetHeight(GetHeightResponse),
    GetTransactions(GetTransactionsResponse),
    GetAltBlocksHashes(GetAltBlocksHashesResponse),
    IsKeyImageSpent(IsKeyImageSpentResponse),
    SendRawTransaction(SendRawTransactionResponse),
    StartMining(StartMiningResponse),
    StopMining(StopMiningResponse),
    MiningStatus(MiningStatusResponse),
    SaveBc(SaveBcResponse),
    GetPeerList(GetPeerListResponse),
    SetLogHashRate(SetLogHashRateResponse),
    SetLogLevel(SetLogLevelResponse),
    SetLogCategories(SetLogCategoriesResponse),
    SetBootstrapDaemon(SetBootstrapDaemonResponse),
    GetTransactionPool(GetTransactionPoolResponse),
    GetTransactionPoolStats(GetTransactionPoolStatsResponse),
    StopDaemon(StopDaemonResponse),
    GetLimit(GetLimitResponse),
    SetLimit(SetLimitResponse),
    OutPeers(OutPeersResponse),
    InPeers(InPeersResponse),
    GetNetStats(GetNetStatsResponse),
    GetOuts(GetOutsResponse),
    Update(UpdateResponse),
    PopBlocks(PopBlocksResponse),
    GetTransactionPoolHashes(GetTransactionPoolHashesResponse),
    GetPublicNodes(GetPublicNodesResponse),
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

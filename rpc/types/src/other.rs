//! JSON types from the [`other`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#other-daemon-rpc-calls) endpoints.
//!
//! All types are originally defined in [`rpc/core_rpc_server_commands_defs.h`](https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h).

//---------------------------------------------------------------------------------------------------- Import
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use cuprate_hex::{Hex, HexVec};
use cuprate_types::rpc::{Peer, PublicNode, TxpoolStats};

use crate::{
    base::{AccessResponseBase, ResponseBase},
    macros::define_request_and_response,
    misc::{GetOutputsOut, OutKey, SpentKeyImageInfo, Status, TxEntry, TxInfo},
    RpcCallValue,
};

#[cfg(any(feature = "serde", feature = "epee"))]
use crate::defaults::{default, default_true};

//---------------------------------------------------------------------------------------------------- Definitions
define_request_and_response! {
    get_height,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 138..=160,
    GetHeight (empty),
    Request {},

    ResponseBase {
        hash: Hex<32>,
        height: u64,
    }
}

define_request_and_response! {
    get_transactions,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 370..=451,
    GetTransactions,

    Request {
        txs_hashes: Vec<Hex<32>>,
        // FIXME: this is documented as optional but it isn't serialized as an optional
        // but it is set _somewhere_ to false in `monerod`
        // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L382>
        decode_as_json: bool = default::<bool>(), "default",
        prune: bool = default::<bool>(), "default",
        split: bool = default::<bool>(), "default",
    },

    AccessResponseBase {
        txs_as_hex: Vec<HexVec>,
        /// `cuprate_rpc_types::json::tx::Transaction` should be used
        /// to create these JSON strings in a type-safe manner.
        txs_as_json: Vec<String>,
        missed_tx: Vec<Hex<32>>,
        txs: Vec<TxEntry>,
    }
}

define_request_and_response! {
    get_alt_blocks_hashes,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 288..=308,
    GetAltBlocksHashes (empty),
    Request {},

    AccessResponseBase {
        blks_hashes: Vec<Hex<32>>,
    }
}

define_request_and_response! {
    is_key_image_spent,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 454..=484,

    IsKeyImageSpent,

    Request {
        key_images: Vec<Hex<32>>,
    },

    AccessResponseBase {
        /// These [`u8`]s are [`cuprate_types::rpc::KeyImageSpentStatus`].
        spent_status: Vec<u8>,
    }
}

define_request_and_response! {
    send_raw_transaction,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 370..=451,

    SendRawTransaction,

    Request {
        tx_as_hex: HexVec,
        do_not_relay: bool = default::<bool>(), "default",
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

    StartMining (restricted),

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
    StopMining (restricted, empty),
    Request {},

    ResponseBase {}
}

define_request_and_response! {
    mining_status,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 846..=895,
    MiningStatus (restricted),
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
    SaveBc (restricted),
    Request {},

    ResponseBase {}
}

define_request_and_response! {
    get_peer_list,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1369..=1417,

    GetPeerList (restricted),

    Request {
        public_only: bool = default_true(), "default_true",
        include_blocked: bool = default::<bool>(), "default",
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

    SetLogHashRate (restricted),

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

    SetLogLevel (restricted),

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

    SetLogCategories (restricted),

    Request {
        categories: String = default::<String>(), "default",
    },

    ResponseBase {
        categories: String,
    }
}

define_request_and_response! {
    set_bootstrap_daemon,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1785..=1812,

    SetBootstrapDaemon (restricted),

    Request {
        address: String,
        username: String = default::<String>(), "default",
        password: String = default::<String>(), "default",
        proxy: String = default::<String>(), "default",
    },

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

    Request {
        limit_down: i64 = default::<i64>(), "default",
        limit_up: i64 = default::<i64>(), "default",
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

    OutPeers (restricted),

    Request {
        set: bool = default_true(), "default_true",
        out_peers: u32,
    },

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

    Update (restricted),

    Request {
        command: String,
        path: String = default::<String>(), "default",
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

    PopBlocks (restricted),


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
    core_rpc_server_commands_defs.h => 1615..=1635,

    GetTransactionPoolHashes (empty),
    Request {},

    ResponseBase {
        tx_hashes: Vec<Hex<32>>,
    }
}

define_request_and_response! {
    UNDOCUMENTED_ENDPOINT,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 1419..=1448,

    GetPublicNodes (restricted),

    Request {
        gray: bool = default::<bool>(), "default",
        white: bool = default_true(), "default_true",
        include_blocked: bool = default::<bool>(), "default",
    },

    ResponseBase {
        gray: Vec<PublicNode>,
        white: Vec<PublicNode>,
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
    use std::fmt::Debug;

    use hex_literal::hex;
    use pretty_assertions::assert_eq;
    use serde::de::DeserializeOwned;

    use cuprate_hex::Hex;
    use cuprate_test_utils::rpc::data::other;
    use cuprate_types::rpc::TxpoolHisto;

    use super::*;

    #[expect(clippy::needless_pass_by_value)]
    fn test_json<T: DeserializeOwned + PartialEq + Debug>(
        cuprate_test_utils_example_data: &str,
        expected_type: T,
    ) {
        let string = serde_json::from_str::<T>(cuprate_test_utils_example_data).unwrap();
        assert_eq!(string, expected_type);
    }

    #[test]
    fn get_height_response() {
        test_json(
            other::GET_HEIGHT_RESPONSE,
            GetHeightResponse {
                base: ResponseBase::OK,
                hash: Hex(hex!(
                    "68bb1a1cff8e2a44c3221e8e1aff80bc6ca45d06fa8eff4d2a3a7ac31d4efe3f"
                )),
                height: 3195160,
            },
        );
    }

    #[test]
    fn get_transactions_request() {
        test_json(
            other::GET_TRANSACTIONS_REQUEST,
            GetTransactionsRequest {
                txs_hashes: vec![Hex(hex!(
                    "d6e48158472848e6687173a91ae6eebfa3e1d778e65252ee99d7515d63090408"
                ))],
                decode_as_json: false,
                prune: false,
                split: false,
            },
        );
    }

    #[test]
    fn get_transactions_response() {
        test_json::<GetTransactionsResponse>(
            other::GET_TRANSACTIONS_RESPONSE,
            GetTransactionsResponse {
                base: AccessResponseBase::OK,
                txs_as_hex: vec![HexVec(hex!("0100940102ffc7afa02501b3056ebee1b651a8da723462b4891d471b990ddc226049a0866d3029b8e2f75b70120280a0b6cef785020190dd0a200bd02b70ee707441a8863c5279b4e4d9f376dc97a140b1e5bc7d72bc5080690280c0caf384a30201d0b12b751e8f6e2e31316110fa6631bf2eb02e88ac8d778ec70d42b24ef54843fd75d90280d0dbc3f40201c498358287895f16b62a000a3f2fd8fb2e70d8e376858fb9ba7d9937d3a076e36311bb0280f092cbdd0801e5a230c6250d5835877b735c71d41587082309bf593d06a78def1b4ec57355a37838b5028080bfb59dd20d01c36c6dd3a9826658642ba4d1d586366f2782c0768c7e9fb93f32e8fdfab18c0228ed0280d0b8e1981a01bfb0158a530682f78754ab5b1b81b15891b2c7a22d4d7a929a5b51c066ffd73ac360230280f092cbdd0801f9a330a1217984cc5d31bf0e76ed4f8e3d4115f470824bc214fa84929fcede137173a60280e0bcefa75701f3910e3a8b3c031e15573f7a69db9f8dda3b3f960253099d8f844169212f2de772f6ff0280d0b8e1981a01adc1157920f2c72d6140afd4b858da3f41d07fc1655f2ebe593d32f96d5335d11711ee0280d0dbc3f40201ca8635a1373fa829f58e8f46d72c8e52aa1ce53fa1d798284ed08b44849e2e9ad79b620280a094a58d1d01faf729e5ab208fa809dd2efc6f0b74d3e7eff2a66c689a3b5c31c33c8a14e2359ac484028080e983b1de1601eced0182c8d37d77ce439824ddb3c8ff7bd60642181e183c409545c9d6f9c36683908f028080d194b57401ead50b3eefebb5303e14a5087de37ad1799a4592cf0e897eafb46d9b57257b5732949e0280a094a58d1d01d3882a1e949b2d1b6fc1fd5e44df95bae9068b090677d76b6c307188da44dd4e343cef028090cad2c60e0196c73a74a60fc4ce3a7b14d1abdf7a0c70a9efb490a9de6ec6208a846f8282d878132b028080bb8b939b4401c03dbcbfd9fb02e181d99c0093e53aceecf42bf6ccc0ec611a5093fe6f2b2738a07f0280f092cbdd0801b98d30c27f297ae4cb89fb7bb29ed11adff17db9b71d39edf736172892784897488c5d0280d0dbc3f40201da9a353d39555c27a2d620bf69136e4c665aaa19557d6fc0255cbb67ec69bf2403b63e0280b09dc2df0101f8820caab7a5e736f5445b5624837de46e9ef906cb538f7c860f688a7f7d155e19e0ac0280808d93f5d77101b544e62708eb27ff140b58c521e4a90acab5eca36f1ce9516a6318306f7d48beddbc0280a0b6cef7850201abdd0a5453712326722427f66b865e67f8cdb7188001aaacb70f1a018403d3289fcb130280c0caf384a30201a2b32b2cfb06b7022668b2cd5b8263a162511c03154b259ce91c6c97270e4c19efe4710280c0caf384a302018db32bda81bfbe5f9cdf94b20047d12a7fb5f097a83099fafdfedc03397826fb4d18d50280c0fc82aa0201b2e60b825e8c0360b4b44f4fe0a30f4d2f18c80d5bbb7bfc5ddf671f27b6867461c51d028080e983b1de1601b2eb0156dd7ab6dcb0970d4a5dbcb4e04281c1db350198e31893cec9b9d77863fedaf60280e08d84ddcb0101e0960fe3cafedb154111449c5112fc1d9e065222ed0243de3207c3e6f974941a66f177028080df9ad7949f01019815c8c5032f2c28e7e6c9f9c70f6fccdece659d8df53e54ad99a0f7fa5d831cf762028090dfc04a01b4fb123d97504b9d832f7041c4d1db1cda3b7a6d307194aff104ec6b711cced2b005e2028080dd9da41701bef1179c9a459e75a0c4cf4aff1a81f31f477bd682e28a155231da1a1aa7a25ef219910280d88ee16f01facd0f043485225a1e708aa89d71f951bc092724b53942a67a35b2315bfeac4e8af0eb0280d0dbc3f40201c4e634d6e1f3a1b232ef130d4a5417379c4fcc9d078f71899f0617cec8b1e72a1844b60280f092cbdd0801f6b9300c8c94a337fefc1c19f12dee0f2551a09ee0aaa954d1762c93fec8dadae2146c0280c0f9decfae0101ce8d09f26c90144257b5462791487fd1b017eb283268b1c86c859c4194bf1a987c62bf0280c0caf384a30201cead2bbb01653d0d7ff8a42958040814c3cbf228ebb772e03956b367bace3b684b9b7f0280a0e5b9c2910101c1b40c1796904ac003f7a6dd72b4845625e99ba12bdd003e65b2dd2760a4e460821178028080e983b1de160186e9013f55160cd9166756ea8e2c9af065dcdfb16a684e9376c909d18b65fd5306f9690280a0e5b9c2910101aeb70c4433f95ff4cdc4aa54a1ede9ae725cec06350db5d3056815486e761e381ae4d00280c0a8ca9a3a01ebe2139bd558b63ebb9f4d12aca270159ccf565e9cffaadd717ce200db779f202b106f0280d0dbc3f40201b9963568acf599958be4e72f71c3446332a39c815876c185198fa2dcf13877eba3627b0280c0f4c198af0b01bccc01408cbb5a210ad152bd6138639673a6161efd2f85be310b477ae14891870985f90280a0b6cef7850201cadd0a82e7950f5d9e62d14d0f7c6af84002ea9822cdeefabbb866b7a5776c6436636b028080d287e2bc2d01d888013a7146b96a7abc5ce5249b7981fb54250eef751964ff00530915084479b5d6ba028080d287e2bc2d018c8901d8cc1933366dceb49416b2f48fd2ce297cfd8da8baadc7f63856c46130368fca0280a0b787e90501b6d242f93a6570ad920332a354b14ad93b37c0f3815bb5fa2dcc7ca5e334256bd165320280a0e5b9c2910101a3ac0c4ed5ebf0c11285c351ddfd0bb52bd225ee0f8a319025dc416a5e2ba8e84186680280c0f9decfae0101f18709daddc52dccb6e1527ac83da15e19c2272206ee0b2ac37ac478b4dd3e6bcac5dc0280f8cce2840201b7c80bc872a1e1323d61342a2a7ac480b4b061163811497e08698057a8774493a1abe50280e0bcefa75701f38b0e532d34791916b1f56c3f008b2231de5cc62bd1ec898c62d19fb1ec716d467ae20280c0fc82aa0201d6da0b7de4dc001430473852620e13e5931960c55ab6ebeff574fbddea995cbc9d7c010280c0f4c198af0b01edca017ec6af4ece2622edaf9914cfb1cc6663639285256912d7d9d70e905120a6961c028090cad2c60e01cad43abcc63a192d53fe8269ecaf2d9ca3171c2172d85956fe44fcc1ac01efe4c610dd0280809aa6eaafe30101a92fccd2bcadfa42dcbd28d483d32abde14b377b72c4e6ef31a1f1e0ff6c2c9f452f0280a0b787e9050197d9420ce413f5321a785cd5bea4952e6c32acd0b733240a2ea2835747bb916a032da7028080d287e2bc2d01c48a01a3c4afcbbdac70524b27585c68ed1f8ea3858c1c392aeb7ada3432f3eed7cab10280f092cbdd0801abb130d362484a00ea63b2a250a8ae7cf8b904522638838a460653a3c37db1b76ff3de0280e08d84ddcb0101cc980fce5c2d8b34b3039e612adeb707f9ab397c75f76c1f0da8af92c64cd021976ff3028080dd9da4170198c217e4a7b63fd645bd27aa5afc6fae7db1e66896cece0d4b84ac76428268b5c213c30280a0b6cef7850201e3e20acab086a758c56372cf461e5339c09745ee510a785b540d68c7f62c35b8b6c5c10280a094a58d1d01ab842ac0ba57e87a3f204a56cbecca405419e4c896c817c5ced715d903a104a09735660280e0a596bb1101ade921f1ef2fe56c74320ceb1f6c52506d0b835808474b928ad6309398b42434d27f3d0280d0b8e1981a01dab515d684a324cff4b00753a9ef0868f308b8121cbc79077ede84d70cf6015ec989be0280c0ee8ed20b01f99c227da8d616ce3669517282902cdf1ef75e75a427385270d1a94197b93cf6620c15028080dd9da41701d2d0175c8494f151e585f01e80c303c84eea460b69874b773ba01d20f28a05916111a0028090dfc04a01a4f312c12a9f52a99f69e43979354446fd4e2ba5e2d5fb8aaa17cd25cdf591543149da0280d0dbc3f40201969c3510fbca0efa6d5b0c45dca32a5a91b10608594a58e5154d6a453493d4a0f10cf70280f8cce2840201ddcb0b76ca6a2df4544ea2d9634223becf72b6d6a176eae609d8a496ee7c0a45bec8240280e0bcefa7570180920e95d8d04f9f7a4b678497ded16da4caca0934fc019f582d8e1db1239920914d35028090cad2c60e01c2d43a30bbb2dcbb2b6c361dc49649a6cf733b29df5f6e7504b03a55ee707ed3db2c4e028080d287e2bc2d01c38801941b46cb00712de68cebc99945dc7b472b352c9a2e582a9217ea6d0b8c3f07590280e0a596bb1101b6eb219463e6e8aa6395eefc4e7d2d7d6484b5f684e7018fe56d3d6ddca82f4b89c5840280d0dbc3f40201db9535db1fb02f4a45c21eae26f5c40c01ab1bca304deac2fb08d2b3d9ac4f65fd10c60280a094a58d1d01948c2a413da2503eb92880f02f764c2133ed6f2951ae86e8c8c17d1e9e024ca4dc72320280c0ee8ed20b01869f22d3e106082527d6f0b052106a4850801fcd59d0b6ce61b237c2321111ed8bdf47028080d194b57401acd20b9c0e61b23698c4b4d47965a597284d409f71d7f16f4997bc04ba042d3cbe044d028090cad2c60e0194b83ac3b448f0bd45f069df6a80e49778c289edeb93b9f213039e53a828e685c270f90280a094a58d1d01bdfb2984b37167dce720d3972eaa50ba42ae1c73ce8e8bc15b5b420e55c9ae96e5ca8c028090dfc04a01abf3120595fbef2082079af5448c6d0d6491aa758576881c1839f4934fa5f6276b33810280e0a596bb1101f9ea2170a571f721540ec01ae22501138fa808045bb8d86b22b1be686b258b2cc999c5028088aca3cf02019cb60d1ffda55346c6612364a9f426a8b9942d9269bef1360f20b8f3ccf57e9996b5f70280e8eda1ba0101aff90c87588ff1bb510a30907357afbf6c3292892c2d9ff41e363889af32e70891cb9b028080d49ca7981201d65ee875df2a98544318a5f4e9aa70a799374b40cff820c132a388736b86ff6c7b7d0280c0caf384a30201dab52bbf532aa44298858b0a313d0f29953ea90efd3ac3421c674dbda79530e4a6b0060280f092cbdd0801c3ab30b0fc9f93dddc6c3e4d976e9c5e4cfee5bfd58415c96a3e7ec05a3172c29f223f0280a094a58d1d01a2812a3e0ec75af0330302c35c582d9a14c8e5f00a0bf84da22eec672c4926ca6fccb10280a094a58d1d01ca842a2b03a22e56f164bae94e43d1c353217c1a1048375731c0c47bb63216e1ef6c480280e08d84ddcb0101d68b0fb2d29505b3f25a8e36f17a2fde13bce41752ecec8c2042a7e1a7d65a0fd35cdf028090cad2c60e0199ce3afa0b62692f1b87324fd4904bf9ffd45ed169d1f5096634a3ba8602919681e5660280c0f9decfae010193ed081977c266b88f1c3cb7027c0216adb506f0e929ce650cd178b81645458c3af4c6028090cad2c60e01eec13a9cce0e6750904e5649907b0bdc6c6963f62fef41ef3932765418d933fc1cd97a0280c0ee8ed20b019ea8228d467474d1073d5c906acdec6ee799b71e16141930c9d66b7f181dbd7a6e924a028080bb8b939b4401c23d3cb4e840ad6766bb0fd6d2b81462f1e4828d2eae341ce3bd8d7ce38b036ac6fb028080e983b1de1601b9e601e3cf485fa441836d83d1f1be6d8611599eccc29f3af832b922e45ab1cd7f31d00280f092cbdd0801fc9e30fff408d7b0c5d88f662dddb5d06ff382baa06191102278e54a0030f7e3246e7c0280d88ee16f01bfcd0f96a24f27ac225278701c6b54df41c6aa511dd86ce682516fb1824ff104c572fb0280f092cbdd0801cbb430bd5f343e45c62efcd6e0f62e77ceb3c95ef945b0cff7002872ea350b5dfffef10280c0caf384a30201bfb22b14dccbba4582da488aef91b530395979f73fa83511e3b3bcb311221c6832b18d0280a0b6cef7850201c4dc0a31fb268316ab21229072833191b7a77b9832afea700a1b93f2e86fb31be9480f028090cad2c60e01cab63a313af96a15c06fcf1f1cf10b69eae50e2c1d005357df457768d384b7a35fa0bb0280d0dbc3f40201fe9835fffd89020879ec3ca02db3eadbb81b0c34a6d77f9d1031438d55fd9c33827db00280d0dbc3f40201d19b354a25ddf2641fc7e000f9306df1c6bf731bddfe9ab148713781bbfab4814ed87e0280e08d84ddcb0101ba960fec80e1dcda9fae2d63e14500354f191f287811f5503e269c9ec1ae56cef4cd110280a0b787e90501acde42b0bdfd00ab0518c8bd6938f0a6efab1b1762704e86c71a154f6d6378dd63ce840280e0bcefa75701b5900eedc2ce12618978351788e46fefe8b9b776510ec32add7423f649c613b9db853a028080e983b1de1601edeb01d68b226cd6b71903a812aa6a6f0799381cf6f70870810df560042cd732b26526028080f696a6b6880101ca18a91fd53d6370a95ca2e0700aabc3784e355afcafb25656c70d780de90e30be31028090cad2c60e0184c03adc307ee3753a20f8f687344aae487639ab12276b604b1f74789d47f1371cac6b0280c0fc82aa0201a2dc0b000aa40a7e3e44d0181aaa5cc64df6307cf119798797fbf82421e3b78a0aa2760280e8eda1ba0101daf20caa8a7c80b400f4dd189e4a00ef1074e26fcc186fed46f0d97814c464aa7561e20280c0f9decfae0101a18b092ee7d48a9fb88cefb22874e5a1ed7a1bf99cc06e93e55c7f75ca4bf38ad185a60280a094a58d1d01dff92904ef53b1415cdb435a1589c072a7e6bd8e69a31bf31153c3beb07ebf585aa838028080bfb59dd20d01916c9d21b580aed847f256b4f507f562858396a9d392adc92b7ed3585d78acf9b38b028080a2a9eae80101fab80b153098181e5fabf1056b4e88db7ce5ed875132e3b7d78ed3b6fc528edda921050280d88ee16f019fcf0fd5f4d68c9afe2e543c125963043024fe557e817c279dbd0602b158fe96ec4b6f0280e0bcefa75701d1910e44b59722c588c30a65b920fc72e0e58c5acc1535b4cad4fc889a89fccfa271510280d0dbc3f40201b78b358b066d485145ada1c39153caacf843fcd9c2f4681d226d210a9a9942109314d90280e0bcefa75701a88b0e5f100858379f9edbbfe92d8f3de825990af354e38edc3b0d246d8a62f01ab3220280d0dbc3f40201959135c6a904269f0bf29fbf8cef1f705dde8c7364ba4618ad9ee378b69a3d590af5680280e0a596bb1101edeb2140e07858aa36619a74b0944c52663b7d3818ab6bf9e66ee792cda1b6dd39842e0280c0a8ca9a3a01aae213a90a6708e741f688a32fb5f1b800800e64cfd341c0f82f8e1ca822336d70c78e0280c0fc82aa02018ddf0b5e03adc078c32952c9077fee655a65a933558c610f23245cd7416669da12611e0280f092cbdd0801aca0305b7157269b35d5068d64f8d43386e8463f2893695bc94f07b4a14f9f5c85e8c50280e0bcefa75701b18f0efd26a0ad840829429252c7e6db2ff0eb7980a8f4c4e400b3a68475f6831cc5f50280b09dc2df0101a6830c2b7555fd29e82d1f0cf6a00f2c671c94c3c683254853c045519d1c5d5dc314fb028080bb8b939b4401be3d76fcfea2c6216513382a75aedaba8932f339ed56f4aad33fb04565429c7f7fa50280c0ee8ed20b01b4a322218a5fd3a13ed0847e8165a28861fb3edc0e2c1603e95e042e2cbb0040e49ab50280c0caf384a30201ecb42b7c10020495d95b3c1ea45ce8709ff4a181771fc053911e5ec51d237d585f19610280f092cbdd0801eba3309533ea103add0540f9624cb24c5cecadf4b93ceb39aa2e541668a0dd23bf3f2f028090dfc04a01a6f3121520ad99387cf4dd779410542b3f5ed9991f0fadbb40f292be0057f4a1dfbf10028090cad2c60e019ac83a125492706ba043a4e3b927ab451c8dccb4b798f83312320dcf4d306bc45c3016028080a2a9eae80101b4ba0bd413da8f7f0aad9cd41d728b4fef20e31fbc61fc397a585c6755134406680b14028080d49ca798120192600ef342c8cf4c0e9ebf52986429add3de7f7757c3d5f7c951810b2fb5352aec620280a0b787e90501afe442797256544eb3515e6fa45b1785d65816dd179bd7f0372a561709f87fae7f95f10280a094a58d1d01dc882aacbd3e13a0b97c2a08b6b6deec5e9685b94409d30c774c85a373b252169d588f028090dfc04a0184f81225e7ded2e83d4f9f0ee64f60c9c8bce2dcb110fd2f3d66c17aafdff53fbf6bbe028080d287e2bc2d01d18901e2fd0eeb4fe9223b4610e05022fcc194240e8afe5472fceda8346cb5b66a0a5902808095e789c60401cf88036cf7317af6dc47cd0ce319a51aaaf936854287c07a24afad791a1431cbd2df5c0280c0f9decfae0101999909d038b9c30a2de009813e56ba2ba17964a24d5f195aaa5f7f2f5fefacd69893e80280a0e5b9c291010199b10cf336c49e2864d07ad3c7a0b9a19e0c17aaf0e72f9fcc980180272000fe5ba1260280a0b6cef7850201a2e20a7a870af412e8fff7eba50b2f8f3be318736996a347fa1222019be9971b6f9b81028090dfc04a01bae5127889a54246328815e9819a05eea4c93bdfffaa2a2cc9747c5d8e74a9a4a8bfe10280f8cce284020191da0b24ee29cd3f554bb618f336dd2841ba23168bf123ee88ebdb48bcbb033a67a02f0280f8cce2840201e6c30b2756e87b0b6ff35103c20c1ddb3b0502f712977fd7909a0b552f1c7dfc3e0c3c028080e983b1de16018fed01a3c245ee280ff115f7e92b16dc2c25831a2da6af5321ad76a1fbbcdd6afc780c0280e0bcefa7570183920ef957193122bb2624d28c0a3cbd4370a1cfff4e1c2e0c8bb22d4c4b47e7f0a5a60280f092cbdd0801ccab30f5440aceabe0c8c408dddf755f789fae2afbf21a64bc183f2d4218a8a792f2870280e08d84ddcb0101f8870f8e26eacca06623c8291d2b29d26ca7f476f09e89c21302d0b85e144267b2712a028080aace938c0901b0b1014c9b9fab49660c2067f4b60151427cf415aa0887447da450652f83a8027524170580b09dc2df01028c792dea94dab48160e067fb681edd6247ba375281fbcfedc03cb970f3b98e2d80b081daaf14021ab33e69737e157d23e33274c42793be06a8711670e73fa10ecebc604f87cc7180a0b6cef78502020752a6308df9466f0838c978216926cb69e113761e84446d5c8453863f06a05c808095e789c60402edc8db59ee3f13d361537cb65c6c89b218e5580a8fbaf9734e9dc71c26a996d780809ce5fd9ed40a024d3ae5019faae01f3e7ae5e978ae0f6a4344976c414b617146f7e76d9c6020c52101038c6d9ccd2f949909115d5321a26e98018b4679138a0a2c06378cf27c8fdbacfd82214a59c99d9251fa00126d353f9cf502a80d8993a6c223e3c802a40ab405555637f495903d3ba558312881e586d452e6e95826d8e128345f6c0a8f9f350e8c04ef50cf34afa3a9ec19c457143496f8cf7045ed869b581f9efa2f1d65e30f1cec5272b00e9c61a34bdd3c78cf82ae8ef4df3132f70861391069b9c255cd0875496ee376e033ee44f8a2d5605a19c88c07f354483a4144f1116143bb212f02fafb5ef7190ce928d2ab32601de56eb944b76935138ca496a345b89b54526a0265614d7932ac0b99289b75b2e18eb4f2918de8cb419bf19d916569b8f90e450bb5bc0da806b7081ecf36da21737ec52379a143632ef483633059741002ee520807713c344b38f710b904c806cf93d3f604111e6565de5f4a64e9d7ea5c24140965684e03cefcb9064ecefb46b82bb5589a6b9baac1800bed502bbc6636ad92026f57fdf2839f36726b0d69615a03b35bb182ec1ef1dcd790a259127a65208e08ea0dd55c8f8cd993c32458562638cf1fb09f77aa7f40e3fecc432f16b2396d0cb7239f7e9f5600bdface5d5f5c0285a9dca1096bd033c4ccf9ceebe063c01e0ec6e2d551189a3d70ae6214a22cd79322de7710ac834c98955d93a5aeed21f900792a98210a1a4a44a17901de0d93e20863a04903e2e77eb119b31c9971653f070ddec02bd08a577bf132323ccf763d6bfc615f1a35802877c6703b70ab7216089a3d5f9b9eacb55ba430484155cb195f736d6c094528b29d3e01032fe61c2c07da6618cf5edad594056db4f6db44adb47721616c4c70e770661634d436e6e90cbcdfdb44603948338401a6ba60c64ca6b51bbf493ecd99ccddd92e6cad20160b0b983744f90cdc4260f60b0776af7c9e664eeb5394ee1182fb6881026271db0a9aad0764782ba106074a0576239681ecae941a9ef56b7b6dda7dbf08ecafac08ab8302d52ee495e4403f2c8b9b18d53ac3863e22d4181688f2bda37943afbf04a436302498f2298b50761eb6e1f43f6354bdc79671b9e97fa239f77924683904e0cf6b1351d4535393a9352d27b007dfda7a8ae8b767e2b5241313d7b5daf20523a80dd6cc9c049da66a5d23f76c132a85d772c45b4c10f2032f58b90c862f09f625cbd18c91a37bb3fc3a413a2e081618da845910cf5b2e6bffea555e883b0bb9c5f9063380a1c33ebdb764d9ffefe9e3169de40b18eeb9bfca48296457bb0b4e29d7b2b5bc4e0021ba0a1d389f77a8e253d6db149d400f675a9330f3bcfd09c7169224a947b6b4e0745ae08cd7adea4151277a94f51f85292ba082cf28300cca233ff4966b093c9cb6abcef476026040fec2b435021aff717b8bb90a40950e010f70bb416a618dc3c5c03f590c5b7ec8e0c05b85ba94078de4817918f783022364b8aa228b5df43b38fba3060c30616f265022584ab6034ddbc832450f90047d0cf41a4af8a20fb1aa66406133a17c2e905ee28d8acd186c872859c196db0474dfaaaded2d63768143cf6b5e2e34662f7bae573a08cb15069ef881892e5a0c08b5c6c7b2e6376cd2080fb29e8d3d5aa5b853662b4f1784ba7f072130e4dc00cba3cc9278fc4213f2ce2fc82bd1ea9fa91bb17b4f7c36962c78d864eab9f30ef327039da6607962a156a05c384a4a58ddd8f51a0d4fe91f64ae7b0a5199110a66f1e676392ec8d31b20a65f7a7fcff90b37a8a3962bff0c83ee6033a70c5b0af663ca48a8f22ced255839444fc51f5b6a6c1237eda5804289aa25fc93f14d0d4a63cecfa30d213eb3b2497af4a22396cc8c0e7c8b8bb57be8878bfc7fb29c038d39cf9fe0c964ebac13354a580527b1dbaced58500a292eb5f7cdafc772860f8d5c324a7079de9e0c1382228effaf2ac0278ebedad1117c5edacf08105a3f0905bca6e59cdf9fd074e1fbb53628a3d9bf3b7be28b33747438a12ae4fed62d035aa49965912839e41d35206a87fff7f79c686584cc23f38277db146dc4bebd0e612edf8b031021e88d1134188cde11bb6ea30883e6a0b0cc38ababe1eb55bf06f26955f25c25c93f40c77f27423131a7769719b09225723dd5283192f74c8a050829fc6fdec46e708111c2bcb1f562a00e831c804fad7a1f74a9be75a7e1720a552f8bd135b6d2b8e8e2a7712b562c33ec9e1030224c0cfc7a6f3b5dc2e6bd02a98d25c73b3a168daa768b70b8aef5bd12f362a89725571c4a82a06d55b22e071a30e15b006a8ea03012d2bb9a7c6a90b7fbd012efbb1c4fa4f35b2a2a2e4f0d54c4e125084208e096cdee54b2763c0f6fbd1f4f341d8829a2d551bfb889e30ca3af81b2fbecc10d0f106845b73475ec5033baab1bad777c23fa55704ba14e01d228597339f3ba6b6caaaa53a8c701b513c4272ff617494277baf9cdea37870ce0d3c03203f93a4ce87b63c577a9d41a7ccbf1c9d0bcdecd8b72a71e9b911b014e172ff24bc63ba064f6fde212df25c40e88257c92f8bc35c4139f058748b00fa511755d9ae5a7b2b2bdf7cdca13b3171ca85a0a1f75c3cae1983c7da7c748076a1c0d2669e7b2e6b71913677af2bc1a21f1c7c436509514320e248015798a050b2cbb1b076cd5eb72cc336d2aad290f959dc6636a050b0811933b01ea25ec006688da1b7e8b4bb963fbe8bc06b5f716a96b15e22be7f8b99b8feba54ac74f080b799ea3a7599daa723067bf837ca32d8921b7584b17d708971fb21cbb8a2808c7da811cff4967363fe7748f0f8378b7c14dd7a5bd10055c78ccb8b8e8b88206317f35dcad0cb2951e5eb7697d484c63764483f7bbc0ad3ca41630fc76a44006e310249d8a73d7f9ca7ce648b5602b331afb584a3e1db1ec9f2a1fc1d030650557c7dbc62008235911677709dea7b60c8d400c9da16b4b0a988b25e5cf26c00c3ef02812def049bb149ea635280e5b339db1035b7275e154b587cc50464a4c0bfd15c79f54faa10fbe571b73cf1aa4a20746b11c80c8c95899521fe5f0bb3104b0a050c55a79511e202fee30c005339694b18f4e18ab5e36ea21952a01864a0e067d9f19362e009a21c6c1a798f7c1325edd95e98fd1f9cb544909fdf9d076070d1233e183fb6d46a46fbc6e10452ef4c45fa0b88a84962ad6e91cbcc52bc000b12a82e93ae5998b20ee9000a8ef68ec8a44862cc108869fd388142692be6b0657e3fe79eff0e8b72f63aeec5874acf5fb0bfc9fa22645ed6ecaaf186eca690ecdf8a71b8f4789ac41b1f4f7539e04c53dd05e67488ea5849bf069d4eefc040273f6018819fdcbaa170c2ce078062b7bbe951d2214b077c4c836db85e1b138059c382ab408a65a3b94132136945cc4a3974c0f96d88eaa1b07cce02dce04ea0126e6210a9543129bb8296839949f6c3867243d4b0e1ff32be58c188ba905d40e32c53f7871920967210de94f71709f73e826036b4e3fa3e42c23f2912f4ea50557dff78aeb34cb35444965614812cbe14068a62be075fce6bf3310b9e8b12e0dd8379104360f728d47a327c172257134e2c0e7c32e01321f4d636f9047bd750e7993eeda7d39fc16f29696b1becee4d8026e967f8149935b947fce8517b2ce02b7831a232f3a29010129c49494ed2b84c7f881b7e4b02a00ebabf5a36023c404002d6cb88cee76c8ce97b03143ca867359d7e118d54e053b02c94998e6fd8409f8d46fc1741a2e56aebb1e7dab7ca3296a2566263d9be2f4bbef4872a49ee1082cbaf86e21b0c232c4182fc660f0c0b6aaeb0393750e553bc406e2a27842bd033da45a562ed1998ef9bd83e35ed813bef00a3e6147cb363bee63c543ba5e770b043dbacc155214a2496f91879bbc9170a2a513d7b48fad40c8c2d96f951e3a0932f6d12956789198430b352803852aa9726163fbe839979b33f8dbf7f76cd50755c1ce0c40a072aeec35057d06abaf59e878000b1d796e51908bfbf23b13900dcb30f9bd52b52994e7245a7017653a404a70d1c444b8c613ff10a2b057c02d062c5faf13cdc4445809fb6e096923cdbbdca18f59318ff86c7e449f596050b404d3cde0338dfdf9b1389178b1d4c70eefa2bbd76fefc1ee1f1688ef507821e40ae31d8d8e673d183b54563e2cbd27e0e042f61b046877d37a68c1b5784830690f2dd4ebbbd2dbdb35800b9e0ba8ea985fa106dd2ce8493e845586716c538ee9008b88a7c482f3c00c14c08468230d40cdc040e145282c4d61985cb5800306e305146204f63e96ad194bcdf1338ab8480341b6fbccf18fc32145f84bece4069c09e41096e94c24fa4f0db988e860a3bff3604143f2b17e8c219f28189e4cd49a0e506fe62dc419299bcd78c6ccb107f63eb31b4bd8ea1e2fed10e3ac17341d3505019e2376b01f7a7fcea3db110fb090c681c866ac86f13e6f8d44a32861e0580def063736b5c771b2b3b9067045867b4393f3eb2a4610bd0216e29906aaac370986451c6bf78264dda7e7a5fcbcf7bd6e024ff6003c6db780d89b97765cee8d0ff3ff25d94d4b4b919f722b26a6903a017daa62af387843087680c57952de06064de05b662af87be49b6e34cf0991cec7be3396e2eec9678ba259bd8de1c192014d02928f9113488215658df4078ed661fa4e79e58decaeb0ee5a00488b094b0b77f083b2b7844f481e7788ffe8004b96ccdf853532bfd9632a8a652c2d97d10173c90864fbb6facf47fae415df4acc0b099140a657b35d083d74dbdfbf107303e74c64471bed4b2199f2babcb4e1fc593d6f309e21f85e68ffd9904731559d0f2b673b36d3984e5d66d897dfa17d601edef3ed78cb70dc5115d4ae240c203e031263f0cf1e98075bac0361fde24cbcb852b8055d53ae01d61a0a1e1ba423d00833747e7364df7ebfd1f84598d801c249e1805279dc37d39fc7f7e27b067e4e0287aec432ed49e4d701a0ff377e88179968430d110cb20476ed4c6bf1624d1907ef24406d3295fcacde2a102cc85f4f3d0cb87a8fae7535a06e442833e58cfc04242ff85fb654d05f9874c0a6756f542db4e9d8b0366191fbb8b09a1bbcb6af04c069978417ca80d92f442b7dbd092f74e1268aa73b54e4b64e84543449ecd30b5ea392a1669a5f441d7208925e91c75df611cd26042630c6b98f160b8c0156048108d5465b71bbc54d31a9f90e34428d97590a427e1ae618d4a35fc1022d4e007c6108dcb1672b88d43ae4d886a5adcc26faf56bc5e5a0b08342fb88263fd80940d1edf794c6ad6d339b974e164b38439e11b4fa87cc793b080b4f8bf0eb56043f79ed3911da21092475fcf8320b55b9f558f194c6c8121b2e696039340d97057be2583726d762b5ae4327e5286a2d8c14ddbe0027c75aacbf7e9de13037390df7d72e13b46bc06bad0363b070e0174d034120d7fa7b4550e7dc28f7f0241f059ae266fc13dccd1d07f744208a7d6a2e565b6613d46e4550f79ef3209c46a805b97284df558719e131f44e419e690f4fc28ee4862b9d1f8f7e1a164ac18141076087693e70ac76a10f7851530d4cbc65def90d5544671ad64249569c3abf0200d09be3c63efaa7cb723b39ccffc9b3a3ba0c8426847123d2a881efbd4937a40cb3e8011c70ba427f80b3dc91a608086f8b61f8bd86fcb482ea388299a7cfbd00a3ddfadb4b6d0e51c1369276c25889a9f3592900b6502d9af1c732e1fb7db307d71e45deb1553ba1568d0480ea9e132b52564da6ac5c56eff823e7f37976cd075ce8f7a78aaef1b87f7437a0b84035677f266f7d0596e493101fec3e14fcf80b22322454587b51fda0636231c07d4e63e007f1b89137d8a37b03bf00f3a7c10169f757d9a74b7bffba797c746e3845decc3d0559d7cf6f08f3bd67dac5f33109b212582dc7df5d561ad63dddc794f2aea4e493db1a73c702d258c9b922c35d04c47f88f87c54c821a29f04abd91a079ce8cef252a21dc72d409fd1618c9be709af029ba98b0140e74666fcb01bced4f88ab68e6b63b8ed6febc0905d22cb2200493c071ce136833697406f8a04e77b21747dda997046cf3c7080096fe481790d77cf5904e7f7128ed95a6e576d109fdf10eb0c888db35a4a685b62253987b70fb1538e6c0932889460fa31c60d123266b7bcb828f846a35b2851127679b05f05a75266529c343a6075e54c455e02b2c83e6f7bf1ae23326506a5f532472d780815c5af425f7d8b543a8f014966e0538f48ca84d181695381a09701eb65c9ae084bf2a4dc84f1b2071be32be25d5f4fcdc59668fd800496ef7eb6dddf867ab908e543cb51f0451706cce4be6b9f68a537a79ea88e17fcd78965b3da68c0d9d30623a2a9e275e1c320f59e118e09c02eee527167bc06f7693e7b584e3b25ecc1093d46b80a1cacced87c2b32e2f90c5bbb9cd1b701aae69a04b16d535fac6eab0d091790fc5fdfa8a8842bfcb62dbf963cbf62c4afb4c468be98c770e6078b8c0a8cfcbae43dcfff17d3c6d587c3e4309fd39c66acd14781fea66fc57278b02302c0fa386280e67acff19955b6a428b0e22ceb1e54e913a37cd19eb6e9d2268a039f2b5fdda7d5804db79385f0e50082b128c952f8dfdedc4411d0675d95127f0bfc01710a869b10d7a8b9e632dad944062567439e6d192fb09329d058e87ecd0aa8981328f541e87ed02cfe4031f2d3a046ff517a2a80486b04ade31a647aec0884fb96ed753ffc47892431c6e6f08fd1c633a1a44e882d3d8b92c567e0fb8305327a354851464ca0f18d89c6ee2a91a4afef0c55883acf8fcb68c2c3b7402e005d8affc19c13f1f26fee0698dff181ab22cb84a2b31e0a6a81dc5d02e60a3c07090397ae58a985526b2ad6ee5725e82328062b68566b4871705ce3b9856e550d068c20fd9aaeb27740c07aad53d79fc20e46e40e7103e2d69626ee64b6aa600f6f1a86f37948ff4990d88f43c34994e2fe586cb779997be323da53329c10480aeb08fe440e9e4b979171371c73b94da9f928a3f6c8f6792f782f3d6432b86d06f54557327fef31fd6ae0a3f6d2f16c9ad947d132e14def33fa24cb4565370e0832fa50f5f5f93c9f3d65776cc22608b68a4f3719e9be47a19432991e4a2c49089c0ea20e7f7c73feaa47970da424d8543d80d622e2f2be9f4c65cc39dc369009a9d41a52bdea7cc0e8e04da87a633fd4f814fda1b646121a469ba0b5b8006d0e9118761d97b5d1856e2d690f27a81c42b176df853d07cf4a66ee83c9eb24ac0a382f5143a10a33ec3ddf17dcd8a8303fac8f279d31b4d04d74bd8804cefbb400c86174ad444e43ed33ee1e1e73f660b9814d5ca3cb1d650f1978a825a617bb05f84eab3b9b8359b991e1084cf4e8179ecb67f92398638e31227ff63427b67f0f232b454a341d85d4b56e31135c9035e231f7d9318ca12b5ab524f87bb0ca9b04b80effed202897ab016d5acc054c4fe62a5f0192f136cf2cd714998a4b164b0c2cdbace52243fdc9ea879b0d247d4fe8bd80481fad6b325cb5f2cfa2534dec0e47d41b6b99352e6e5faccb5ee28ca2fe96e04f9c83a0461ba34cfb499d864f05dc734b6c8f51cc0c994290b2a868cb8312df39fb6d457a81e62d872c65d4f3007094be42663bca3d64ebbcc8401158fce4f5a38a49c20f029c338f126451820459866e77c6984a467aad571cc7452392a9cb9f8e65099fff2f2acd170a833e01ed3d22a683356ee42dcbe6bab6d05d3edda2d40e9ba53884d430c2e0cd87c0067dc8cb68c868bd9f29db1dd73703c139ffc15e4f7264e727c70560ae02da100871f30e8a7c1275805f621752d73aafecddc2a7808b6c2ecbb8d0134a644bb603f30f8d18b3fc5efaa7f206ce180bfb14f5dbd3b0115145a227113eeaf1c1ec04244227b931388e72960833a40baa4319c5cf74aa94f7e3233e1d09f0a4f74409999684ad1cc836ac74563c85b164664dfab08ea084b25e2cbd7e7b94a781a10fcd455ee38b65126dcc52016127fd195c80b05660ed931e128b0cb92868955c0d032ada9fb951210a5442d21c1718ebc4702ad4a57967e15a63ffb05e1e072a0c41ebdf1e7205373eeaf4587f695de887fa3a8c96b8deb99e040fa1fc4dc2a402a017891943d734ae2f3798b22b1269d3d9f6d65581b9c637a6896a4fb554810bbd3db5c5737391a74150b43413b2e3824490b7911cbeb845147f1a8521620b0dd31306f13a9754a01bcdbd18bfdeade06b0ec97f48df56c45d3670a1fe18d00ef13e613c8a77aeb40401a814b377137cf44f29cb2cb94186ad1161ecb05a7c07837a5ab3474e57990cff2ab16b4d99f62e646da28e8bb712a5b561cf0e25be039c3e08583c8ebc3dd2fdb8fdc6e135ecc7851c73218a70b75e697cc84ea50504b9c34a33ed52f87230b9d192a940f3b7bb6d45b58dbf52f0afeb8dac85c77b06bdf9b70a10cb81c50055c9d8cf7e3a5c4b7dfae55beabcb3e8a8a1cb822d8d0bf6c01e32056929f853021eae6c97fdb0c5031df6b2e7c57f1318866769a9cc09c38ed62d8bf4663334c0df67c47236ed73f6ce7f54e0ada9270398c1aa558d0f993b0d25d97aea77b1635ee4832362cd590bae5fc1549402ddcd42b15efc930111a01535c0242116078d6d2d53b8612d378c4370e90d0d01b01bd7da591bec07981652a98485d8ed5c8f3def2bdac7d992ee5fc6a1ec7bd36940e1bc58c7050451248fc3ee6069e6b1b0d3ef122c6ef2a9b99aa0f145fb43341c58dbb472130b51730c956273a3ef6df9e000f6a87c2bacdefcdb5daef28b6170f61bc3a9c101f439755c86e6b85ee06a7a60688b3843eb359cd4acd9221a2ee131e2fd2e190652e5c47c0b98c41010eb99a991ec48a5de99cc8f403d6d76f8307d6657c1e007ebd64eec7bbd0d4f1ba2db7bb0efe27c7828f053e00def775943ab01a7e33d0fffcfe6f9a7285237f2c381b638758e373f8ceac672190664bb25fb5d355c240bd1773d61bda7f7ef1f4261b80ff5058ec6f7e024ab9459b1103815624b81f80c39db2f6fecb72de452b11636b0f71b16cb55f883d93bebb94328f13ef1ab6d0df449e32d27884f5139af584035547dace65ee25ba05cc461e74760d4468af90dcaa982e52cb902e2b84b3324019da575601ca54e91655913892e703257deaa01d14fd8459ff780c724161ba4d4280b70a5039dcfb5d775560714009724cb0d0b7e178c71e777b896bcfcde7d4c9c3dc6ab819d74a1a1fda8486448b1ad79be02fb134ea93a8600f1bc2a42e68d0213ab461a07cef3ad3965bc130beb76bab409102f82bf6c4cd626f6df3388e17b87584310c50832cde3191f6557f0014bdc0a68d924119e43111043bc6f26d16a5f2612dae6ab24984e2d87a71d93d5f4670dba2176d4f16633407bf7c10b51b6842dfdbc6fe3eaa4b6a12f0550700ece070ca382dec3b587e0e1fc317a48a83754d15aaf9a6971b8cb641fd8b32846d89002e6301700a0e7056e8002d8f269d29ebaf64f4493b1f1e676fc78e673067fb00625df15dc0490235b386ee14e55b335f3bc6dcedd7d3a80fd3a6e9bc2ccf3af0d89be71b5ca92bd7a9b97b9ff8976f75702419aa5bf9be34600496ca1bfa8ad0400602a23579365574252434f2bcd7efb360b0e8a495e8f7e78923b6fbf2207049e9179f0d4d7d6b4a4a10ca10f0ef4dd6cb5a74f7574e832044d6120fbc1580a68eddfbc65ab300bed960a6f24a102dc36b72937a8be4385daf5946e81ccde0619251babbff17e5685217a134d22f6130d0322483b3475227ffd27adc73ca202a6debfa37e5731747f4449ac70a33684f460eede65918c6d89acf4b50fd28d040ffbd436a944d3be0210606bfc2301e7ac66d462dba29a0489eb55af714a760e5302592cccc726e535b945ceb6126eb84e31f0f140ff54df8be0fa3a22f418036ad996787a5616a97a42049ebce351dc11857cab3dc914ef26833b0e75653004a8cafea099fb0750135255c41ef43e2f29c75714e2f0be2545e7c109b70c43004a471daa85b47befc65907d033f133b2f3ac2ad568df630ee80506610b8dc9052d442668dc06b13ea76ab1ab7b34870341d660af5d3007c21bb72512e4f8a60d8916a037b93f9e15ac9e4a6a1246d73ebb40e5fdd5a0d6dc0cf175023b891301f69fe5a3ca6f12cb8312d16333de1cd3ebb99339ab18c0715bfcd35b8365b407ad759e2c591d8270ad335381573e27ec18af7ca157b4a2bbca921db083d9b0009dc332a79dde14354a8c18bce76a1bfc1a25a1e702ccaa0feb521ee9279b8a01ceab6e237bbe4128b23cb53b1e5185f3266e20670a307ea0cfb5377025e0bb0790d48f1636c8b836c1a1f69ad61265f19057197e86cd526da6ddb94fd1ece80b60852f27ef2ce56ccb5a32d8cab6d16be06f380dfde3602ea4c1ae927173b2001ff0d9e29bc66b2b2a20c3e3ac174fcba187aacab0876c1356d30d4021e6dd0048c3bdfbf254108bb09d3ca9f2be423a92408bca52fbcd68f972c46fc8d20e0350d12c2f2d6c7da85e96bcec3ce61119793d44a210f81ece859fef6360ae3b0e1af0634fc141a8b50b3b383fb264e8a4fb84ea06db6becbf5e140edf66ee190da8968da579eb349fedea45e4c252a79570501278bab5fa984d7b1179d7c2460faa7beafee153bbae0a591701632aa94839528d3ef50cf809c1f7209b9e5c99010eaff7f921c45b6546358ee7a90948e3c710cd3e1796860839a345516fdf4f07c415029627abe1273a1f510c36a662562d18169b23305b4efadfefbfbb41a400ab533e61c14cafa49bc5d2818058ee4f3e1aeb329e150820d1de1f1eaaad31051a6dfdd3a1d5cec7b16bc0ea2c649d409917faa42138b1f824b4d534a050be0a99ea6772daf0b2e58623cc7a250ef37599bd556508f08886e663ef0917ecd3077072c3268ea5b9b89cbb6b761ee9f9c4765d8b267d9eb19728a28ce67a42ed0cad142b5dc0fc5313853860ec3f0ee2bc3d47cbe12dbe9633db809967d5b8bf0e45574eac657059530c30aeeade1e4f858a4a6e79d6e441b4af0127a13340d908d48cfec849ee93d53b1564231f048d34885e791a9d40c61a7b00f12f6f72a5050bcaabcc98480170ea6e20bef6b5c6f504c808108454fe2f3c275bf8f89a5e0a3304a7c4787e6d4fbe569930f7cfd38ab7d1d2ebd599bbb411950cec3e53b90cefb82234990d353c71ce4c21ef674a1c4070f71c90e1ea7edf35f5a421118f01b49a92ea97720e2d4df6b5885c181002656629a90eaa1904fe1c379b8291480ca15d0dc2b65a20c22f1e01d612d21ecb5738e5ebfc578a4a65066ee6e913e3030d3fdfb0168fd75022492728ee82869deb9ff2827f4e10759ecddb20f67e9808e707257a74d3dc0a6068f264066f95c9f772a3dcec0b4f0a327e3745517ad60ccbc5392890d2479b724d068fcdb83607e02291c06e1a5a1dac7604889cce2500f418da2f7080a7e9a1bdf28b87028a2bbb0c14f059f10f46d46716eac2cdfc06676cbec91b8c2c0f7c9bea7e27fa5048662398b23a9b488a49e1d3330c04e60179a4492c8b836780899899d2af17e6119a94d54a890ce8c0b550e87fd54cba0821fa7c48f6e09a60dcbddf853f82b47195aa44a5ceae14a9257296acd711c8073ff3345befca5d3ebf64901b283df96395fe9785d7090176bfe5a9f13ceae701c6c93af0e13d949bc3c7e9b06674a73e7affc508302258a27fb34569c3742201c0721aef282a31c69a5d98a67ac5c3d920d50c089896f7f8c8c237a81f803f0444f417246d695e89a3a523b62a3cd2203d42607cac7c7782dec1f9edbb806c0a7a37d1a969082a126bc726151a50233456a07d374399e74aaa8cc66821511d092615950d302e815cfcc021e1250cdea20fd9e1e4b5e88280d6e4283b918e780d12cbba59ef2ed2ce86135a48fba6c0dc2bf2efee190d9a3f9aa22a622b1953058f2bb3a371637d13e045d54e7eb54c0d25851f49283d7d34e9785d2d5c3f70086c48a8325a2083bdf5b3531fcc697cc0c9f63892a866c84585d673a2a63fd60e77995bfb0c0a44a4b63c0ff67e813027d3e84cddd393a0f4e6bc95525c5ae20eed9d0cea4a12aa748eb5209cfd75990b055f1ad0472f9f7599f569a8743a720755aa11555df4bb2e725fa93bc5dea603a964e8dc9fb1742e81825022866fc50a6b2a19b6a234a38ee27a74f2f5832b294143ca7ff8d07fd7d4e01f479e9792058871d90ee3aaa3329e82cebe41dff5e6d00a36268a7965466b80c6510ac1350cee797e1d6737f6aaff155266d2a2d611b2124affed1ac73a6a06515627b2230ce0d7fed33ecbde511f4d472cbc556cc8d9c5640e67657035112976b626847a0a4ea5fdd14d4a3eed57f0dfbe153393d8bd28c8b4f9e62940e8379790393fa20c617050c780a7d870193b4611bd7a12d26947a3cf4605e225da8b1646a76984015a5e317016a4d8301eaeec0db3ae0daa719182e2f4479154dbcccfcce1f365099de6c91934c395ce82abba8062a51d7773b418330921766cd3d275c689098e06039698db6f09accb292e7eb79e7a022d4257bb2f9ed993c519860919bc229a06ad88954c9ebf7f5b9fe95cf56e8181cb9175dac06be0be70fd28df20cdb4600ef0869668c645c9ea01360fdae7c922cb3d2b3583ae1de5ae7d899a83ff2bb00d7365c782a0fccfcba7f87bb29416469bb051f9b0755123e0f2fa76dc7644b70e452f49a84bc372b384c843b8161b7f9b63699adcadd5cb2b33b36c7eb3e1b00f25218bc16447968b939016242fceaebd796c17a24d1b9870991a9c3ae90e380302b7bb320adacc08cfb9249d29cd9275c52476dac6a7e9870ee3776cbc3352036f9c8f681d44856c6c5f90b7cde0877472ddd48719c449f59dca1f49442f7505e4809c6d323b37530ecccf3e41e19822f53d64dc90efb113405ee88799c37f0a342293b5bfc019a9057138326de6107b5613554dffc737aed7237fb16cd77e09f581d12220ac930c6ca279efd1d07a92125fb2606ec3ec35351987a15fc72806cfb3cb66fce8dcfabee5c1e586bf0f802fa12ae5ad5a708e3a5d54e1926dbd0202bf1150f1bb612b9a4590b5b520b86a90860ec3d9c2184f9975ced15ae1300882d9918021b43a1184ba88ddd7091539fe5a7017b8708d0f5c916f9c42de5103f8116863864b508f5880ca60b7492385c16a02b6ceb64d257a4838873b85d2041517c5c7c4508e4d5a5faa72729d73af0361e11828eeca992b8f20d903a5ef065976a9f322e34bd4b3984bb09e18be40e77e833c8c1a2e80093227d3f40d4a067f5e3aee9fce9bd234bb6ff4d0c34fc060d23e86b1f5a6d8d052e53e913182052a2d9c5e97bb0e0a51bb2fafbe7346bacfcbadb00ce2ba129f29d41a11f7d105cf19bb60b5f5b0dfd6a894698ef7f56a02d69cc03eb62a56563d3a77e3ac2302").into())],
                txs_as_json: vec![],
                missed_tx: vec![],
                txs: vec![
                    TxEntry {
                        as_json: String::new(),
                        as_hex: HexVec(hex!("0100940102ffc7afa02501b3056ebee1b651a8da723462b4891d471b990ddc226049a0866d3029b8e2f75b70120280a0b6cef785020190dd0a200bd02b70ee707441a8863c5279b4e4d9f376dc97a140b1e5bc7d72bc5080690280c0caf384a30201d0b12b751e8f6e2e31316110fa6631bf2eb02e88ac8d778ec70d42b24ef54843fd75d90280d0dbc3f40201c498358287895f16b62a000a3f2fd8fb2e70d8e376858fb9ba7d9937d3a076e36311bb0280f092cbdd0801e5a230c6250d5835877b735c71d41587082309bf593d06a78def1b4ec57355a37838b5028080bfb59dd20d01c36c6dd3a9826658642ba4d1d586366f2782c0768c7e9fb93f32e8fdfab18c0228ed0280d0b8e1981a01bfb0158a530682f78754ab5b1b81b15891b2c7a22d4d7a929a5b51c066ffd73ac360230280f092cbdd0801f9a330a1217984cc5d31bf0e76ed4f8e3d4115f470824bc214fa84929fcede137173a60280e0bcefa75701f3910e3a8b3c031e15573f7a69db9f8dda3b3f960253099d8f844169212f2de772f6ff0280d0b8e1981a01adc1157920f2c72d6140afd4b858da3f41d07fc1655f2ebe593d32f96d5335d11711ee0280d0dbc3f40201ca8635a1373fa829f58e8f46d72c8e52aa1ce53fa1d798284ed08b44849e2e9ad79b620280a094a58d1d01faf729e5ab208fa809dd2efc6f0b74d3e7eff2a66c689a3b5c31c33c8a14e2359ac484028080e983b1de1601eced0182c8d37d77ce439824ddb3c8ff7bd60642181e183c409545c9d6f9c36683908f028080d194b57401ead50b3eefebb5303e14a5087de37ad1799a4592cf0e897eafb46d9b57257b5732949e0280a094a58d1d01d3882a1e949b2d1b6fc1fd5e44df95bae9068b090677d76b6c307188da44dd4e343cef028090cad2c60e0196c73a74a60fc4ce3a7b14d1abdf7a0c70a9efb490a9de6ec6208a846f8282d878132b028080bb8b939b4401c03dbcbfd9fb02e181d99c0093e53aceecf42bf6ccc0ec611a5093fe6f2b2738a07f0280f092cbdd0801b98d30c27f297ae4cb89fb7bb29ed11adff17db9b71d39edf736172892784897488c5d0280d0dbc3f40201da9a353d39555c27a2d620bf69136e4c665aaa19557d6fc0255cbb67ec69bf2403b63e0280b09dc2df0101f8820caab7a5e736f5445b5624837de46e9ef906cb538f7c860f688a7f7d155e19e0ac0280808d93f5d77101b544e62708eb27ff140b58c521e4a90acab5eca36f1ce9516a6318306f7d48beddbc0280a0b6cef7850201abdd0a5453712326722427f66b865e67f8cdb7188001aaacb70f1a018403d3289fcb130280c0caf384a30201a2b32b2cfb06b7022668b2cd5b8263a162511c03154b259ce91c6c97270e4c19efe4710280c0caf384a302018db32bda81bfbe5f9cdf94b20047d12a7fb5f097a83099fafdfedc03397826fb4d18d50280c0fc82aa0201b2e60b825e8c0360b4b44f4fe0a30f4d2f18c80d5bbb7bfc5ddf671f27b6867461c51d028080e983b1de1601b2eb0156dd7ab6dcb0970d4a5dbcb4e04281c1db350198e31893cec9b9d77863fedaf60280e08d84ddcb0101e0960fe3cafedb154111449c5112fc1d9e065222ed0243de3207c3e6f974941a66f177028080df9ad7949f01019815c8c5032f2c28e7e6c9f9c70f6fccdece659d8df53e54ad99a0f7fa5d831cf762028090dfc04a01b4fb123d97504b9d832f7041c4d1db1cda3b7a6d307194aff104ec6b711cced2b005e2028080dd9da41701bef1179c9a459e75a0c4cf4aff1a81f31f477bd682e28a155231da1a1aa7a25ef219910280d88ee16f01facd0f043485225a1e708aa89d71f951bc092724b53942a67a35b2315bfeac4e8af0eb0280d0dbc3f40201c4e634d6e1f3a1b232ef130d4a5417379c4fcc9d078f71899f0617cec8b1e72a1844b60280f092cbdd0801f6b9300c8c94a337fefc1c19f12dee0f2551a09ee0aaa954d1762c93fec8dadae2146c0280c0f9decfae0101ce8d09f26c90144257b5462791487fd1b017eb283268b1c86c859c4194bf1a987c62bf0280c0caf384a30201cead2bbb01653d0d7ff8a42958040814c3cbf228ebb772e03956b367bace3b684b9b7f0280a0e5b9c2910101c1b40c1796904ac003f7a6dd72b4845625e99ba12bdd003e65b2dd2760a4e460821178028080e983b1de160186e9013f55160cd9166756ea8e2c9af065dcdfb16a684e9376c909d18b65fd5306f9690280a0e5b9c2910101aeb70c4433f95ff4cdc4aa54a1ede9ae725cec06350db5d3056815486e761e381ae4d00280c0a8ca9a3a01ebe2139bd558b63ebb9f4d12aca270159ccf565e9cffaadd717ce200db779f202b106f0280d0dbc3f40201b9963568acf599958be4e72f71c3446332a39c815876c185198fa2dcf13877eba3627b0280c0f4c198af0b01bccc01408cbb5a210ad152bd6138639673a6161efd2f85be310b477ae14891870985f90280a0b6cef7850201cadd0a82e7950f5d9e62d14d0f7c6af84002ea9822cdeefabbb866b7a5776c6436636b028080d287e2bc2d01d888013a7146b96a7abc5ce5249b7981fb54250eef751964ff00530915084479b5d6ba028080d287e2bc2d018c8901d8cc1933366dceb49416b2f48fd2ce297cfd8da8baadc7f63856c46130368fca0280a0b787e90501b6d242f93a6570ad920332a354b14ad93b37c0f3815bb5fa2dcc7ca5e334256bd165320280a0e5b9c2910101a3ac0c4ed5ebf0c11285c351ddfd0bb52bd225ee0f8a319025dc416a5e2ba8e84186680280c0f9decfae0101f18709daddc52dccb6e1527ac83da15e19c2272206ee0b2ac37ac478b4dd3e6bcac5dc0280f8cce2840201b7c80bc872a1e1323d61342a2a7ac480b4b061163811497e08698057a8774493a1abe50280e0bcefa75701f38b0e532d34791916b1f56c3f008b2231de5cc62bd1ec898c62d19fb1ec716d467ae20280c0fc82aa0201d6da0b7de4dc001430473852620e13e5931960c55ab6ebeff574fbddea995cbc9d7c010280c0f4c198af0b01edca017ec6af4ece2622edaf9914cfb1cc6663639285256912d7d9d70e905120a6961c028090cad2c60e01cad43abcc63a192d53fe8269ecaf2d9ca3171c2172d85956fe44fcc1ac01efe4c610dd0280809aa6eaafe30101a92fccd2bcadfa42dcbd28d483d32abde14b377b72c4e6ef31a1f1e0ff6c2c9f452f0280a0b787e9050197d9420ce413f5321a785cd5bea4952e6c32acd0b733240a2ea2835747bb916a032da7028080d287e2bc2d01c48a01a3c4afcbbdac70524b27585c68ed1f8ea3858c1c392aeb7ada3432f3eed7cab10280f092cbdd0801abb130d362484a00ea63b2a250a8ae7cf8b904522638838a460653a3c37db1b76ff3de0280e08d84ddcb0101cc980fce5c2d8b34b3039e612adeb707f9ab397c75f76c1f0da8af92c64cd021976ff3028080dd9da4170198c217e4a7b63fd645bd27aa5afc6fae7db1e66896cece0d4b84ac76428268b5c213c30280a0b6cef7850201e3e20acab086a758c56372cf461e5339c09745ee510a785b540d68c7f62c35b8b6c5c10280a094a58d1d01ab842ac0ba57e87a3f204a56cbecca405419e4c896c817c5ced715d903a104a09735660280e0a596bb1101ade921f1ef2fe56c74320ceb1f6c52506d0b835808474b928ad6309398b42434d27f3d0280d0b8e1981a01dab515d684a324cff4b00753a9ef0868f308b8121cbc79077ede84d70cf6015ec989be0280c0ee8ed20b01f99c227da8d616ce3669517282902cdf1ef75e75a427385270d1a94197b93cf6620c15028080dd9da41701d2d0175c8494f151e585f01e80c303c84eea460b69874b773ba01d20f28a05916111a0028090dfc04a01a4f312c12a9f52a99f69e43979354446fd4e2ba5e2d5fb8aaa17cd25cdf591543149da0280d0dbc3f40201969c3510fbca0efa6d5b0c45dca32a5a91b10608594a58e5154d6a453493d4a0f10cf70280f8cce2840201ddcb0b76ca6a2df4544ea2d9634223becf72b6d6a176eae609d8a496ee7c0a45bec8240280e0bcefa7570180920e95d8d04f9f7a4b678497ded16da4caca0934fc019f582d8e1db1239920914d35028090cad2c60e01c2d43a30bbb2dcbb2b6c361dc49649a6cf733b29df5f6e7504b03a55ee707ed3db2c4e028080d287e2bc2d01c38801941b46cb00712de68cebc99945dc7b472b352c9a2e582a9217ea6d0b8c3f07590280e0a596bb1101b6eb219463e6e8aa6395eefc4e7d2d7d6484b5f684e7018fe56d3d6ddca82f4b89c5840280d0dbc3f40201db9535db1fb02f4a45c21eae26f5c40c01ab1bca304deac2fb08d2b3d9ac4f65fd10c60280a094a58d1d01948c2a413da2503eb92880f02f764c2133ed6f2951ae86e8c8c17d1e9e024ca4dc72320280c0ee8ed20b01869f22d3e106082527d6f0b052106a4850801fcd59d0b6ce61b237c2321111ed8bdf47028080d194b57401acd20b9c0e61b23698c4b4d47965a597284d409f71d7f16f4997bc04ba042d3cbe044d028090cad2c60e0194b83ac3b448f0bd45f069df6a80e49778c289edeb93b9f213039e53a828e685c270f90280a094a58d1d01bdfb2984b37167dce720d3972eaa50ba42ae1c73ce8e8bc15b5b420e55c9ae96e5ca8c028090dfc04a01abf3120595fbef2082079af5448c6d0d6491aa758576881c1839f4934fa5f6276b33810280e0a596bb1101f9ea2170a571f721540ec01ae22501138fa808045bb8d86b22b1be686b258b2cc999c5028088aca3cf02019cb60d1ffda55346c6612364a9f426a8b9942d9269bef1360f20b8f3ccf57e9996b5f70280e8eda1ba0101aff90c87588ff1bb510a30907357afbf6c3292892c2d9ff41e363889af32e70891cb9b028080d49ca7981201d65ee875df2a98544318a5f4e9aa70a799374b40cff820c132a388736b86ff6c7b7d0280c0caf384a30201dab52bbf532aa44298858b0a313d0f29953ea90efd3ac3421c674dbda79530e4a6b0060280f092cbdd0801c3ab30b0fc9f93dddc6c3e4d976e9c5e4cfee5bfd58415c96a3e7ec05a3172c29f223f0280a094a58d1d01a2812a3e0ec75af0330302c35c582d9a14c8e5f00a0bf84da22eec672c4926ca6fccb10280a094a58d1d01ca842a2b03a22e56f164bae94e43d1c353217c1a1048375731c0c47bb63216e1ef6c480280e08d84ddcb0101d68b0fb2d29505b3f25a8e36f17a2fde13bce41752ecec8c2042a7e1a7d65a0fd35cdf028090cad2c60e0199ce3afa0b62692f1b87324fd4904bf9ffd45ed169d1f5096634a3ba8602919681e5660280c0f9decfae010193ed081977c266b88f1c3cb7027c0216adb506f0e929ce650cd178b81645458c3af4c6028090cad2c60e01eec13a9cce0e6750904e5649907b0bdc6c6963f62fef41ef3932765418d933fc1cd97a0280c0ee8ed20b019ea8228d467474d1073d5c906acdec6ee799b71e16141930c9d66b7f181dbd7a6e924a028080bb8b939b4401c23d3cb4e840ad6766bb0fd6d2b81462f1e4828d2eae341ce3bd8d7ce38b036ac6fb028080e983b1de1601b9e601e3cf485fa441836d83d1f1be6d8611599eccc29f3af832b922e45ab1cd7f31d00280f092cbdd0801fc9e30fff408d7b0c5d88f662dddb5d06ff382baa06191102278e54a0030f7e3246e7c0280d88ee16f01bfcd0f96a24f27ac225278701c6b54df41c6aa511dd86ce682516fb1824ff104c572fb0280f092cbdd0801cbb430bd5f343e45c62efcd6e0f62e77ceb3c95ef945b0cff7002872ea350b5dfffef10280c0caf384a30201bfb22b14dccbba4582da488aef91b530395979f73fa83511e3b3bcb311221c6832b18d0280a0b6cef7850201c4dc0a31fb268316ab21229072833191b7a77b9832afea700a1b93f2e86fb31be9480f028090cad2c60e01cab63a313af96a15c06fcf1f1cf10b69eae50e2c1d005357df457768d384b7a35fa0bb0280d0dbc3f40201fe9835fffd89020879ec3ca02db3eadbb81b0c34a6d77f9d1031438d55fd9c33827db00280d0dbc3f40201d19b354a25ddf2641fc7e000f9306df1c6bf731bddfe9ab148713781bbfab4814ed87e0280e08d84ddcb0101ba960fec80e1dcda9fae2d63e14500354f191f287811f5503e269c9ec1ae56cef4cd110280a0b787e90501acde42b0bdfd00ab0518c8bd6938f0a6efab1b1762704e86c71a154f6d6378dd63ce840280e0bcefa75701b5900eedc2ce12618978351788e46fefe8b9b776510ec32add7423f649c613b9db853a028080e983b1de1601edeb01d68b226cd6b71903a812aa6a6f0799381cf6f70870810df560042cd732b26526028080f696a6b6880101ca18a91fd53d6370a95ca2e0700aabc3784e355afcafb25656c70d780de90e30be31028090cad2c60e0184c03adc307ee3753a20f8f687344aae487639ab12276b604b1f74789d47f1371cac6b0280c0fc82aa0201a2dc0b000aa40a7e3e44d0181aaa5cc64df6307cf119798797fbf82421e3b78a0aa2760280e8eda1ba0101daf20caa8a7c80b400f4dd189e4a00ef1074e26fcc186fed46f0d97814c464aa7561e20280c0f9decfae0101a18b092ee7d48a9fb88cefb22874e5a1ed7a1bf99cc06e93e55c7f75ca4bf38ad185a60280a094a58d1d01dff92904ef53b1415cdb435a1589c072a7e6bd8e69a31bf31153c3beb07ebf585aa838028080bfb59dd20d01916c9d21b580aed847f256b4f507f562858396a9d392adc92b7ed3585d78acf9b38b028080a2a9eae80101fab80b153098181e5fabf1056b4e88db7ce5ed875132e3b7d78ed3b6fc528edda921050280d88ee16f019fcf0fd5f4d68c9afe2e543c125963043024fe557e817c279dbd0602b158fe96ec4b6f0280e0bcefa75701d1910e44b59722c588c30a65b920fc72e0e58c5acc1535b4cad4fc889a89fccfa271510280d0dbc3f40201b78b358b066d485145ada1c39153caacf843fcd9c2f4681d226d210a9a9942109314d90280e0bcefa75701a88b0e5f100858379f9edbbfe92d8f3de825990af354e38edc3b0d246d8a62f01ab3220280d0dbc3f40201959135c6a904269f0bf29fbf8cef1f705dde8c7364ba4618ad9ee378b69a3d590af5680280e0a596bb1101edeb2140e07858aa36619a74b0944c52663b7d3818ab6bf9e66ee792cda1b6dd39842e0280c0a8ca9a3a01aae213a90a6708e741f688a32fb5f1b800800e64cfd341c0f82f8e1ca822336d70c78e0280c0fc82aa02018ddf0b5e03adc078c32952c9077fee655a65a933558c610f23245cd7416669da12611e0280f092cbdd0801aca0305b7157269b35d5068d64f8d43386e8463f2893695bc94f07b4a14f9f5c85e8c50280e0bcefa75701b18f0efd26a0ad840829429252c7e6db2ff0eb7980a8f4c4e400b3a68475f6831cc5f50280b09dc2df0101a6830c2b7555fd29e82d1f0cf6a00f2c671c94c3c683254853c045519d1c5d5dc314fb028080bb8b939b4401be3d76fcfea2c6216513382a75aedaba8932f339ed56f4aad33fb04565429c7f7fa50280c0ee8ed20b01b4a322218a5fd3a13ed0847e8165a28861fb3edc0e2c1603e95e042e2cbb0040e49ab50280c0caf384a30201ecb42b7c10020495d95b3c1ea45ce8709ff4a181771fc053911e5ec51d237d585f19610280f092cbdd0801eba3309533ea103add0540f9624cb24c5cecadf4b93ceb39aa2e541668a0dd23bf3f2f028090dfc04a01a6f3121520ad99387cf4dd779410542b3f5ed9991f0fadbb40f292be0057f4a1dfbf10028090cad2c60e019ac83a125492706ba043a4e3b927ab451c8dccb4b798f83312320dcf4d306bc45c3016028080a2a9eae80101b4ba0bd413da8f7f0aad9cd41d728b4fef20e31fbc61fc397a585c6755134406680b14028080d49ca798120192600ef342c8cf4c0e9ebf52986429add3de7f7757c3d5f7c951810b2fb5352aec620280a0b787e90501afe442797256544eb3515e6fa45b1785d65816dd179bd7f0372a561709f87fae7f95f10280a094a58d1d01dc882aacbd3e13a0b97c2a08b6b6deec5e9685b94409d30c774c85a373b252169d588f028090dfc04a0184f81225e7ded2e83d4f9f0ee64f60c9c8bce2dcb110fd2f3d66c17aafdff53fbf6bbe028080d287e2bc2d01d18901e2fd0eeb4fe9223b4610e05022fcc194240e8afe5472fceda8346cb5b66a0a5902808095e789c60401cf88036cf7317af6dc47cd0ce319a51aaaf936854287c07a24afad791a1431cbd2df5c0280c0f9decfae0101999909d038b9c30a2de009813e56ba2ba17964a24d5f195aaa5f7f2f5fefacd69893e80280a0e5b9c291010199b10cf336c49e2864d07ad3c7a0b9a19e0c17aaf0e72f9fcc980180272000fe5ba1260280a0b6cef7850201a2e20a7a870af412e8fff7eba50b2f8f3be318736996a347fa1222019be9971b6f9b81028090dfc04a01bae5127889a54246328815e9819a05eea4c93bdfffaa2a2cc9747c5d8e74a9a4a8bfe10280f8cce284020191da0b24ee29cd3f554bb618f336dd2841ba23168bf123ee88ebdb48bcbb033a67a02f0280f8cce2840201e6c30b2756e87b0b6ff35103c20c1ddb3b0502f712977fd7909a0b552f1c7dfc3e0c3c028080e983b1de16018fed01a3c245ee280ff115f7e92b16dc2c25831a2da6af5321ad76a1fbbcdd6afc780c0280e0bcefa7570183920ef957193122bb2624d28c0a3cbd4370a1cfff4e1c2e0c8bb22d4c4b47e7f0a5a60280f092cbdd0801ccab30f5440aceabe0c8c408dddf755f789fae2afbf21a64bc183f2d4218a8a792f2870280e08d84ddcb0101f8870f8e26eacca06623c8291d2b29d26ca7f476f09e89c21302d0b85e144267b2712a028080aace938c0901b0b1014c9b9fab49660c2067f4b60151427cf415aa0887447da450652f83a8027524170580b09dc2df01028c792dea94dab48160e067fb681edd6247ba375281fbcfedc03cb970f3b98e2d80b081daaf14021ab33e69737e157d23e33274c42793be06a8711670e73fa10ecebc604f87cc7180a0b6cef78502020752a6308df9466f0838c978216926cb69e113761e84446d5c8453863f06a05c808095e789c60402edc8db59ee3f13d361537cb65c6c89b218e5580a8fbaf9734e9dc71c26a996d780809ce5fd9ed40a024d3ae5019faae01f3e7ae5e978ae0f6a4344976c414b617146f7e76d9c6020c52101038c6d9ccd2f949909115d5321a26e98018b4679138a0a2c06378cf27c8fdbacfd82214a59c99d9251fa00126d353f9cf502a80d8993a6c223e3c802a40ab405555637f495903d3ba558312881e586d452e6e95826d8e128345f6c0a8f9f350e8c04ef50cf34afa3a9ec19c457143496f8cf7045ed869b581f9efa2f1d65e30f1cec5272b00e9c61a34bdd3c78cf82ae8ef4df3132f70861391069b9c255cd0875496ee376e033ee44f8a2d5605a19c88c07f354483a4144f1116143bb212f02fafb5ef7190ce928d2ab32601de56eb944b76935138ca496a345b89b54526a0265614d7932ac0b99289b75b2e18eb4f2918de8cb419bf19d916569b8f90e450bb5bc0da806b7081ecf36da21737ec52379a143632ef483633059741002ee520807713c344b38f710b904c806cf93d3f604111e6565de5f4a64e9d7ea5c24140965684e03cefcb9064ecefb46b82bb5589a6b9baac1800bed502bbc6636ad92026f57fdf2839f36726b0d69615a03b35bb182ec1ef1dcd790a259127a65208e08ea0dd55c8f8cd993c32458562638cf1fb09f77aa7f40e3fecc432f16b2396d0cb7239f7e9f5600bdface5d5f5c0285a9dca1096bd033c4ccf9ceebe063c01e0ec6e2d551189a3d70ae6214a22cd79322de7710ac834c98955d93a5aeed21f900792a98210a1a4a44a17901de0d93e20863a04903e2e77eb119b31c9971653f070ddec02bd08a577bf132323ccf763d6bfc615f1a35802877c6703b70ab7216089a3d5f9b9eacb55ba430484155cb195f736d6c094528b29d3e01032fe61c2c07da6618cf5edad594056db4f6db44adb47721616c4c70e770661634d436e6e90cbcdfdb44603948338401a6ba60c64ca6b51bbf493ecd99ccddd92e6cad20160b0b983744f90cdc4260f60b0776af7c9e664eeb5394ee1182fb6881026271db0a9aad0764782ba106074a0576239681ecae941a9ef56b7b6dda7dbf08ecafac08ab8302d52ee495e4403f2c8b9b18d53ac3863e22d4181688f2bda37943afbf04a436302498f2298b50761eb6e1f43f6354bdc79671b9e97fa239f77924683904e0cf6b1351d4535393a9352d27b007dfda7a8ae8b767e2b5241313d7b5daf20523a80dd6cc9c049da66a5d23f76c132a85d772c45b4c10f2032f58b90c862f09f625cbd18c91a37bb3fc3a413a2e081618da845910cf5b2e6bffea555e883b0bb9c5f9063380a1c33ebdb764d9ffefe9e3169de40b18eeb9bfca48296457bb0b4e29d7b2b5bc4e0021ba0a1d389f77a8e253d6db149d400f675a9330f3bcfd09c7169224a947b6b4e0745ae08cd7adea4151277a94f51f85292ba082cf28300cca233ff4966b093c9cb6abcef476026040fec2b435021aff717b8bb90a40950e010f70bb416a618dc3c5c03f590c5b7ec8e0c05b85ba94078de4817918f783022364b8aa228b5df43b38fba3060c30616f265022584ab6034ddbc832450f90047d0cf41a4af8a20fb1aa66406133a17c2e905ee28d8acd186c872859c196db0474dfaaaded2d63768143cf6b5e2e34662f7bae573a08cb15069ef881892e5a0c08b5c6c7b2e6376cd2080fb29e8d3d5aa5b853662b4f1784ba7f072130e4dc00cba3cc9278fc4213f2ce2fc82bd1ea9fa91bb17b4f7c36962c78d864eab9f30ef327039da6607962a156a05c384a4a58ddd8f51a0d4fe91f64ae7b0a5199110a66f1e676392ec8d31b20a65f7a7fcff90b37a8a3962bff0c83ee6033a70c5b0af663ca48a8f22ced255839444fc51f5b6a6c1237eda5804289aa25fc93f14d0d4a63cecfa30d213eb3b2497af4a22396cc8c0e7c8b8bb57be8878bfc7fb29c038d39cf9fe0c964ebac13354a580527b1dbaced58500a292eb5f7cdafc772860f8d5c324a7079de9e0c1382228effaf2ac0278ebedad1117c5edacf08105a3f0905bca6e59cdf9fd074e1fbb53628a3d9bf3b7be28b33747438a12ae4fed62d035aa49965912839e41d35206a87fff7f79c686584cc23f38277db146dc4bebd0e612edf8b031021e88d1134188cde11bb6ea30883e6a0b0cc38ababe1eb55bf06f26955f25c25c93f40c77f27423131a7769719b09225723dd5283192f74c8a050829fc6fdec46e708111c2bcb1f562a00e831c804fad7a1f74a9be75a7e1720a552f8bd135b6d2b8e8e2a7712b562c33ec9e1030224c0cfc7a6f3b5dc2e6bd02a98d25c73b3a168daa768b70b8aef5bd12f362a89725571c4a82a06d55b22e071a30e15b006a8ea03012d2bb9a7c6a90b7fbd012efbb1c4fa4f35b2a2a2e4f0d54c4e125084208e096cdee54b2763c0f6fbd1f4f341d8829a2d551bfb889e30ca3af81b2fbecc10d0f106845b73475ec5033baab1bad777c23fa55704ba14e01d228597339f3ba6b6caaaa53a8c701b513c4272ff617494277baf9cdea37870ce0d3c03203f93a4ce87b63c577a9d41a7ccbf1c9d0bcdecd8b72a71e9b911b014e172ff24bc63ba064f6fde212df25c40e88257c92f8bc35c4139f058748b00fa511755d9ae5a7b2b2bdf7cdca13b3171ca85a0a1f75c3cae1983c7da7c748076a1c0d2669e7b2e6b71913677af2bc1a21f1c7c436509514320e248015798a050b2cbb1b076cd5eb72cc336d2aad290f959dc6636a050b0811933b01ea25ec006688da1b7e8b4bb963fbe8bc06b5f716a96b15e22be7f8b99b8feba54ac74f080b799ea3a7599daa723067bf837ca32d8921b7584b17d708971fb21cbb8a2808c7da811cff4967363fe7748f0f8378b7c14dd7a5bd10055c78ccb8b8e8b88206317f35dcad0cb2951e5eb7697d484c63764483f7bbc0ad3ca41630fc76a44006e310249d8a73d7f9ca7ce648b5602b331afb584a3e1db1ec9f2a1fc1d030650557c7dbc62008235911677709dea7b60c8d400c9da16b4b0a988b25e5cf26c00c3ef02812def049bb149ea635280e5b339db1035b7275e154b587cc50464a4c0bfd15c79f54faa10fbe571b73cf1aa4a20746b11c80c8c95899521fe5f0bb3104b0a050c55a79511e202fee30c005339694b18f4e18ab5e36ea21952a01864a0e067d9f19362e009a21c6c1a798f7c1325edd95e98fd1f9cb544909fdf9d076070d1233e183fb6d46a46fbc6e10452ef4c45fa0b88a84962ad6e91cbcc52bc000b12a82e93ae5998b20ee9000a8ef68ec8a44862cc108869fd388142692be6b0657e3fe79eff0e8b72f63aeec5874acf5fb0bfc9fa22645ed6ecaaf186eca690ecdf8a71b8f4789ac41b1f4f7539e04c53dd05e67488ea5849bf069d4eefc040273f6018819fdcbaa170c2ce078062b7bbe951d2214b077c4c836db85e1b138059c382ab408a65a3b94132136945cc4a3974c0f96d88eaa1b07cce02dce04ea0126e6210a9543129bb8296839949f6c3867243d4b0e1ff32be58c188ba905d40e32c53f7871920967210de94f71709f73e826036b4e3fa3e42c23f2912f4ea50557dff78aeb34cb35444965614812cbe14068a62be075fce6bf3310b9e8b12e0dd8379104360f728d47a327c172257134e2c0e7c32e01321f4d636f9047bd750e7993eeda7d39fc16f29696b1becee4d8026e967f8149935b947fce8517b2ce02b7831a232f3a29010129c49494ed2b84c7f881b7e4b02a00ebabf5a36023c404002d6cb88cee76c8ce97b03143ca867359d7e118d54e053b02c94998e6fd8409f8d46fc1741a2e56aebb1e7dab7ca3296a2566263d9be2f4bbef4872a49ee1082cbaf86e21b0c232c4182fc660f0c0b6aaeb0393750e553bc406e2a27842bd033da45a562ed1998ef9bd83e35ed813bef00a3e6147cb363bee63c543ba5e770b043dbacc155214a2496f91879bbc9170a2a513d7b48fad40c8c2d96f951e3a0932f6d12956789198430b352803852aa9726163fbe839979b33f8dbf7f76cd50755c1ce0c40a072aeec35057d06abaf59e878000b1d796e51908bfbf23b13900dcb30f9bd52b52994e7245a7017653a404a70d1c444b8c613ff10a2b057c02d062c5faf13cdc4445809fb6e096923cdbbdca18f59318ff86c7e449f596050b404d3cde0338dfdf9b1389178b1d4c70eefa2bbd76fefc1ee1f1688ef507821e40ae31d8d8e673d183b54563e2cbd27e0e042f61b046877d37a68c1b5784830690f2dd4ebbbd2dbdb35800b9e0ba8ea985fa106dd2ce8493e845586716c538ee9008b88a7c482f3c00c14c08468230d40cdc040e145282c4d61985cb5800306e305146204f63e96ad194bcdf1338ab8480341b6fbccf18fc32145f84bece4069c09e41096e94c24fa4f0db988e860a3bff3604143f2b17e8c219f28189e4cd49a0e506fe62dc419299bcd78c6ccb107f63eb31b4bd8ea1e2fed10e3ac17341d3505019e2376b01f7a7fcea3db110fb090c681c866ac86f13e6f8d44a32861e0580def063736b5c771b2b3b9067045867b4393f3eb2a4610bd0216e29906aaac370986451c6bf78264dda7e7a5fcbcf7bd6e024ff6003c6db780d89b97765cee8d0ff3ff25d94d4b4b919f722b26a6903a017daa62af387843087680c57952de06064de05b662af87be49b6e34cf0991cec7be3396e2eec9678ba259bd8de1c192014d02928f9113488215658df4078ed661fa4e79e58decaeb0ee5a00488b094b0b77f083b2b7844f481e7788ffe8004b96ccdf853532bfd9632a8a652c2d97d10173c90864fbb6facf47fae415df4acc0b099140a657b35d083d74dbdfbf107303e74c64471bed4b2199f2babcb4e1fc593d6f309e21f85e68ffd9904731559d0f2b673b36d3984e5d66d897dfa17d601edef3ed78cb70dc5115d4ae240c203e031263f0cf1e98075bac0361fde24cbcb852b8055d53ae01d61a0a1e1ba423d00833747e7364df7ebfd1f84598d801c249e1805279dc37d39fc7f7e27b067e4e0287aec432ed49e4d701a0ff377e88179968430d110cb20476ed4c6bf1624d1907ef24406d3295fcacde2a102cc85f4f3d0cb87a8fae7535a06e442833e58cfc04242ff85fb654d05f9874c0a6756f542db4e9d8b0366191fbb8b09a1bbcb6af04c069978417ca80d92f442b7dbd092f74e1268aa73b54e4b64e84543449ecd30b5ea392a1669a5f441d7208925e91c75df611cd26042630c6b98f160b8c0156048108d5465b71bbc54d31a9f90e34428d97590a427e1ae618d4a35fc1022d4e007c6108dcb1672b88d43ae4d886a5adcc26faf56bc5e5a0b08342fb88263fd80940d1edf794c6ad6d339b974e164b38439e11b4fa87cc793b080b4f8bf0eb56043f79ed3911da21092475fcf8320b55b9f558f194c6c8121b2e696039340d97057be2583726d762b5ae4327e5286a2d8c14ddbe0027c75aacbf7e9de13037390df7d72e13b46bc06bad0363b070e0174d034120d7fa7b4550e7dc28f7f0241f059ae266fc13dccd1d07f744208a7d6a2e565b6613d46e4550f79ef3209c46a805b97284df558719e131f44e419e690f4fc28ee4862b9d1f8f7e1a164ac18141076087693e70ac76a10f7851530d4cbc65def90d5544671ad64249569c3abf0200d09be3c63efaa7cb723b39ccffc9b3a3ba0c8426847123d2a881efbd4937a40cb3e8011c70ba427f80b3dc91a608086f8b61f8bd86fcb482ea388299a7cfbd00a3ddfadb4b6d0e51c1369276c25889a9f3592900b6502d9af1c732e1fb7db307d71e45deb1553ba1568d0480ea9e132b52564da6ac5c56eff823e7f37976cd075ce8f7a78aaef1b87f7437a0b84035677f266f7d0596e493101fec3e14fcf80b22322454587b51fda0636231c07d4e63e007f1b89137d8a37b03bf00f3a7c10169f757d9a74b7bffba797c746e3845decc3d0559d7cf6f08f3bd67dac5f33109b212582dc7df5d561ad63dddc794f2aea4e493db1a73c702d258c9b922c35d04c47f88f87c54c821a29f04abd91a079ce8cef252a21dc72d409fd1618c9be709af029ba98b0140e74666fcb01bced4f88ab68e6b63b8ed6febc0905d22cb2200493c071ce136833697406f8a04e77b21747dda997046cf3c7080096fe481790d77cf5904e7f7128ed95a6e576d109fdf10eb0c888db35a4a685b62253987b70fb1538e6c0932889460fa31c60d123266b7bcb828f846a35b2851127679b05f05a75266529c343a6075e54c455e02b2c83e6f7bf1ae23326506a5f532472d780815c5af425f7d8b543a8f014966e0538f48ca84d181695381a09701eb65c9ae084bf2a4dc84f1b2071be32be25d5f4fcdc59668fd800496ef7eb6dddf867ab908e543cb51f0451706cce4be6b9f68a537a79ea88e17fcd78965b3da68c0d9d30623a2a9e275e1c320f59e118e09c02eee527167bc06f7693e7b584e3b25ecc1093d46b80a1cacced87c2b32e2f90c5bbb9cd1b701aae69a04b16d535fac6eab0d091790fc5fdfa8a8842bfcb62dbf963cbf62c4afb4c468be98c770e6078b8c0a8cfcbae43dcfff17d3c6d587c3e4309fd39c66acd14781fea66fc57278b02302c0fa386280e67acff19955b6a428b0e22ceb1e54e913a37cd19eb6e9d2268a039f2b5fdda7d5804db79385f0e50082b128c952f8dfdedc4411d0675d95127f0bfc01710a869b10d7a8b9e632dad944062567439e6d192fb09329d058e87ecd0aa8981328f541e87ed02cfe4031f2d3a046ff517a2a80486b04ade31a647aec0884fb96ed753ffc47892431c6e6f08fd1c633a1a44e882d3d8b92c567e0fb8305327a354851464ca0f18d89c6ee2a91a4afef0c55883acf8fcb68c2c3b7402e005d8affc19c13f1f26fee0698dff181ab22cb84a2b31e0a6a81dc5d02e60a3c07090397ae58a985526b2ad6ee5725e82328062b68566b4871705ce3b9856e550d068c20fd9aaeb27740c07aad53d79fc20e46e40e7103e2d69626ee64b6aa600f6f1a86f37948ff4990d88f43c34994e2fe586cb779997be323da53329c10480aeb08fe440e9e4b979171371c73b94da9f928a3f6c8f6792f782f3d6432b86d06f54557327fef31fd6ae0a3f6d2f16c9ad947d132e14def33fa24cb4565370e0832fa50f5f5f93c9f3d65776cc22608b68a4f3719e9be47a19432991e4a2c49089c0ea20e7f7c73feaa47970da424d8543d80d622e2f2be9f4c65cc39dc369009a9d41a52bdea7cc0e8e04da87a633fd4f814fda1b646121a469ba0b5b8006d0e9118761d97b5d1856e2d690f27a81c42b176df853d07cf4a66ee83c9eb24ac0a382f5143a10a33ec3ddf17dcd8a8303fac8f279d31b4d04d74bd8804cefbb400c86174ad444e43ed33ee1e1e73f660b9814d5ca3cb1d650f1978a825a617bb05f84eab3b9b8359b991e1084cf4e8179ecb67f92398638e31227ff63427b67f0f232b454a341d85d4b56e31135c9035e231f7d9318ca12b5ab524f87bb0ca9b04b80effed202897ab016d5acc054c4fe62a5f0192f136cf2cd714998a4b164b0c2cdbace52243fdc9ea879b0d247d4fe8bd80481fad6b325cb5f2cfa2534dec0e47d41b6b99352e6e5faccb5ee28ca2fe96e04f9c83a0461ba34cfb499d864f05dc734b6c8f51cc0c994290b2a868cb8312df39fb6d457a81e62d872c65d4f3007094be42663bca3d64ebbcc8401158fce4f5a38a49c20f029c338f126451820459866e77c6984a467aad571cc7452392a9cb9f8e65099fff2f2acd170a833e01ed3d22a683356ee42dcbe6bab6d05d3edda2d40e9ba53884d430c2e0cd87c0067dc8cb68c868bd9f29db1dd73703c139ffc15e4f7264e727c70560ae02da100871f30e8a7c1275805f621752d73aafecddc2a7808b6c2ecbb8d0134a644bb603f30f8d18b3fc5efaa7f206ce180bfb14f5dbd3b0115145a227113eeaf1c1ec04244227b931388e72960833a40baa4319c5cf74aa94f7e3233e1d09f0a4f74409999684ad1cc836ac74563c85b164664dfab08ea084b25e2cbd7e7b94a781a10fcd455ee38b65126dcc52016127fd195c80b05660ed931e128b0cb92868955c0d032ada9fb951210a5442d21c1718ebc4702ad4a57967e15a63ffb05e1e072a0c41ebdf1e7205373eeaf4587f695de887fa3a8c96b8deb99e040fa1fc4dc2a402a017891943d734ae2f3798b22b1269d3d9f6d65581b9c637a6896a4fb554810bbd3db5c5737391a74150b43413b2e3824490b7911cbeb845147f1a8521620b0dd31306f13a9754a01bcdbd18bfdeade06b0ec97f48df56c45d3670a1fe18d00ef13e613c8a77aeb40401a814b377137cf44f29cb2cb94186ad1161ecb05a7c07837a5ab3474e57990cff2ab16b4d99f62e646da28e8bb712a5b561cf0e25be039c3e08583c8ebc3dd2fdb8fdc6e135ecc7851c73218a70b75e697cc84ea50504b9c34a33ed52f87230b9d192a940f3b7bb6d45b58dbf52f0afeb8dac85c77b06bdf9b70a10cb81c50055c9d8cf7e3a5c4b7dfae55beabcb3e8a8a1cb822d8d0bf6c01e32056929f853021eae6c97fdb0c5031df6b2e7c57f1318866769a9cc09c38ed62d8bf4663334c0df67c47236ed73f6ce7f54e0ada9270398c1aa558d0f993b0d25d97aea77b1635ee4832362cd590bae5fc1549402ddcd42b15efc930111a01535c0242116078d6d2d53b8612d378c4370e90d0d01b01bd7da591bec07981652a98485d8ed5c8f3def2bdac7d992ee5fc6a1ec7bd36940e1bc58c7050451248fc3ee6069e6b1b0d3ef122c6ef2a9b99aa0f145fb43341c58dbb472130b51730c956273a3ef6df9e000f6a87c2bacdefcdb5daef28b6170f61bc3a9c101f439755c86e6b85ee06a7a60688b3843eb359cd4acd9221a2ee131e2fd2e190652e5c47c0b98c41010eb99a991ec48a5de99cc8f403d6d76f8307d6657c1e007ebd64eec7bbd0d4f1ba2db7bb0efe27c7828f053e00def775943ab01a7e33d0fffcfe6f9a7285237f2c381b638758e373f8ceac672190664bb25fb5d355c240bd1773d61bda7f7ef1f4261b80ff5058ec6f7e024ab9459b1103815624b81f80c39db2f6fecb72de452b11636b0f71b16cb55f883d93bebb94328f13ef1ab6d0df449e32d27884f5139af584035547dace65ee25ba05cc461e74760d4468af90dcaa982e52cb902e2b84b3324019da575601ca54e91655913892e703257deaa01d14fd8459ff780c724161ba4d4280b70a5039dcfb5d775560714009724cb0d0b7e178c71e777b896bcfcde7d4c9c3dc6ab819d74a1a1fda8486448b1ad79be02fb134ea93a8600f1bc2a42e68d0213ab461a07cef3ad3965bc130beb76bab409102f82bf6c4cd626f6df3388e17b87584310c50832cde3191f6557f0014bdc0a68d924119e43111043bc6f26d16a5f2612dae6ab24984e2d87a71d93d5f4670dba2176d4f16633407bf7c10b51b6842dfdbc6fe3eaa4b6a12f0550700ece070ca382dec3b587e0e1fc317a48a83754d15aaf9a6971b8cb641fd8b32846d89002e6301700a0e7056e8002d8f269d29ebaf64f4493b1f1e676fc78e673067fb00625df15dc0490235b386ee14e55b335f3bc6dcedd7d3a80fd3a6e9bc2ccf3af0d89be71b5ca92bd7a9b97b9ff8976f75702419aa5bf9be34600496ca1bfa8ad0400602a23579365574252434f2bcd7efb360b0e8a495e8f7e78923b6fbf2207049e9179f0d4d7d6b4a4a10ca10f0ef4dd6cb5a74f7574e832044d6120fbc1580a68eddfbc65ab300bed960a6f24a102dc36b72937a8be4385daf5946e81ccde0619251babbff17e5685217a134d22f6130d0322483b3475227ffd27adc73ca202a6debfa37e5731747f4449ac70a33684f460eede65918c6d89acf4b50fd28d040ffbd436a944d3be0210606bfc2301e7ac66d462dba29a0489eb55af714a760e5302592cccc726e535b945ceb6126eb84e31f0f140ff54df8be0fa3a22f418036ad996787a5616a97a42049ebce351dc11857cab3dc914ef26833b0e75653004a8cafea099fb0750135255c41ef43e2f29c75714e2f0be2545e7c109b70c43004a471daa85b47befc65907d033f133b2f3ac2ad568df630ee80506610b8dc9052d442668dc06b13ea76ab1ab7b34870341d660af5d3007c21bb72512e4f8a60d8916a037b93f9e15ac9e4a6a1246d73ebb40e5fdd5a0d6dc0cf175023b891301f69fe5a3ca6f12cb8312d16333de1cd3ebb99339ab18c0715bfcd35b8365b407ad759e2c591d8270ad335381573e27ec18af7ca157b4a2bbca921db083d9b0009dc332a79dde14354a8c18bce76a1bfc1a25a1e702ccaa0feb521ee9279b8a01ceab6e237bbe4128b23cb53b1e5185f3266e20670a307ea0cfb5377025e0bb0790d48f1636c8b836c1a1f69ad61265f19057197e86cd526da6ddb94fd1ece80b60852f27ef2ce56ccb5a32d8cab6d16be06f380dfde3602ea4c1ae927173b2001ff0d9e29bc66b2b2a20c3e3ac174fcba187aacab0876c1356d30d4021e6dd0048c3bdfbf254108bb09d3ca9f2be423a92408bca52fbcd68f972c46fc8d20e0350d12c2f2d6c7da85e96bcec3ce61119793d44a210f81ece859fef6360ae3b0e1af0634fc141a8b50b3b383fb264e8a4fb84ea06db6becbf5e140edf66ee190da8968da579eb349fedea45e4c252a79570501278bab5fa984d7b1179d7c2460faa7beafee153bbae0a591701632aa94839528d3ef50cf809c1f7209b9e5c99010eaff7f921c45b6546358ee7a90948e3c710cd3e1796860839a345516fdf4f07c415029627abe1273a1f510c36a662562d18169b23305b4efadfefbfbb41a400ab533e61c14cafa49bc5d2818058ee4f3e1aeb329e150820d1de1f1eaaad31051a6dfdd3a1d5cec7b16bc0ea2c649d409917faa42138b1f824b4d534a050be0a99ea6772daf0b2e58623cc7a250ef37599bd556508f08886e663ef0917ecd3077072c3268ea5b9b89cbb6b761ee9f9c4765d8b267d9eb19728a28ce67a42ed0cad142b5dc0fc5313853860ec3f0ee2bc3d47cbe12dbe9633db809967d5b8bf0e45574eac657059530c30aeeade1e4f858a4a6e79d6e441b4af0127a13340d908d48cfec849ee93d53b1564231f048d34885e791a9d40c61a7b00f12f6f72a5050bcaabcc98480170ea6e20bef6b5c6f504c808108454fe2f3c275bf8f89a5e0a3304a7c4787e6d4fbe569930f7cfd38ab7d1d2ebd599bbb411950cec3e53b90cefb82234990d353c71ce4c21ef674a1c4070f71c90e1ea7edf35f5a421118f01b49a92ea97720e2d4df6b5885c181002656629a90eaa1904fe1c379b8291480ca15d0dc2b65a20c22f1e01d612d21ecb5738e5ebfc578a4a65066ee6e913e3030d3fdfb0168fd75022492728ee82869deb9ff2827f4e10759ecddb20f67e9808e707257a74d3dc0a6068f264066f95c9f772a3dcec0b4f0a327e3745517ad60ccbc5392890d2479b724d068fcdb83607e02291c06e1a5a1dac7604889cce2500f418da2f7080a7e9a1bdf28b87028a2bbb0c14f059f10f46d46716eac2cdfc06676cbec91b8c2c0f7c9bea7e27fa5048662398b23a9b488a49e1d3330c04e60179a4492c8b836780899899d2af17e6119a94d54a890ce8c0b550e87fd54cba0821fa7c48f6e09a60dcbddf853f82b47195aa44a5ceae14a9257296acd711c8073ff3345befca5d3ebf64901b283df96395fe9785d7090176bfe5a9f13ceae701c6c93af0e13d949bc3c7e9b06674a73e7affc508302258a27fb34569c3742201c0721aef282a31c69a5d98a67ac5c3d920d50c089896f7f8c8c237a81f803f0444f417246d695e89a3a523b62a3cd2203d42607cac7c7782dec1f9edbb806c0a7a37d1a969082a126bc726151a50233456a07d374399e74aaa8cc66821511d092615950d302e815cfcc021e1250cdea20fd9e1e4b5e88280d6e4283b918e780d12cbba59ef2ed2ce86135a48fba6c0dc2bf2efee190d9a3f9aa22a622b1953058f2bb3a371637d13e045d54e7eb54c0d25851f49283d7d34e9785d2d5c3f70086c48a8325a2083bdf5b3531fcc697cc0c9f63892a866c84585d673a2a63fd60e77995bfb0c0a44a4b63c0ff67e813027d3e84cddd393a0f4e6bc95525c5ae20eed9d0cea4a12aa748eb5209cfd75990b055f1ad0472f9f7599f569a8743a720755aa11555df4bb2e725fa93bc5dea603a964e8dc9fb1742e81825022866fc50a6b2a19b6a234a38ee27a74f2f5832b294143ca7ff8d07fd7d4e01f479e9792058871d90ee3aaa3329e82cebe41dff5e6d00a36268a7965466b80c6510ac1350cee797e1d6737f6aaff155266d2a2d611b2124affed1ac73a6a06515627b2230ce0d7fed33ecbde511f4d472cbc556cc8d9c5640e67657035112976b626847a0a4ea5fdd14d4a3eed57f0dfbe153393d8bd28c8b4f9e62940e8379790393fa20c617050c780a7d870193b4611bd7a12d26947a3cf4605e225da8b1646a76984015a5e317016a4d8301eaeec0db3ae0daa719182e2f4479154dbcccfcce1f365099de6c91934c395ce82abba8062a51d7773b418330921766cd3d275c689098e06039698db6f09accb292e7eb79e7a022d4257bb2f9ed993c519860919bc229a06ad88954c9ebf7f5b9fe95cf56e8181cb9175dac06be0be70fd28df20cdb4600ef0869668c645c9ea01360fdae7c922cb3d2b3583ae1de5ae7d899a83ff2bb00d7365c782a0fccfcba7f87bb29416469bb051f9b0755123e0f2fa76dc7644b70e452f49a84bc372b384c843b8161b7f9b63699adcadd5cb2b33b36c7eb3e1b00f25218bc16447968b939016242fceaebd796c17a24d1b9870991a9c3ae90e380302b7bb320adacc08cfb9249d29cd9275c52476dac6a7e9870ee3776cbc3352036f9c8f681d44856c6c5f90b7cde0877472ddd48719c449f59dca1f49442f7505e4809c6d323b37530ecccf3e41e19822f53d64dc90efb113405ee88799c37f0a342293b5bfc019a9057138326de6107b5613554dffc737aed7237fb16cd77e09f581d12220ac930c6ca279efd1d07a92125fb2606ec3ec35351987a15fc72806cfb3cb66fce8dcfabee5c1e586bf0f802fa12ae5ad5a708e3a5d54e1926dbd0202bf1150f1bb612b9a4590b5b520b86a90860ec3d9c2184f9975ced15ae1300882d9918021b43a1184ba88ddd7091539fe5a7017b8708d0f5c916f9c42de5103f8116863864b508f5880ca60b7492385c16a02b6ceb64d257a4838873b85d2041517c5c7c4508e4d5a5faa72729d73af0361e11828eeca992b8f20d903a5ef065976a9f322e34bd4b3984bb09e18be40e77e833c8c1a2e80093227d3f40d4a067f5e3aee9fce9bd234bb6ff4d0c34fc060d23e86b1f5a6d8d052e53e913182052a2d9c5e97bb0e0a51bb2fafbe7346bacfcbadb00ce2ba129f29d41a11f7d105cf19bb60b5f5b0dfd6a894698ef7f56a02d69cc03eb62a56563d3a77e3ac2302").into()),
                        double_spend_seen: false,
                        tx_hash: Hex(hex!("d6e48158472848e6687173a91ae6eebfa3e1d778e65252ee99d7515d63090408")),
                        prunable_as_hex: HexVec::new(),
                        prunable_hash: Hex(hex!("0000000000000000000000000000000000000000000000000000000000000000")),
                        pruned_as_hex: HexVec::new(),
                        tx_entry_type: crate::misc::TxEntryType::Blockchain { block_height: 993442, block_timestamp: 1457749396, confirmations: 2201720, output_indices: vec![        198769,
        418598,
        176616,
        50345,
        509
], in_pool: false },
                    }
                ],
            },
        );
    }

    #[test]
    fn get_alt_blocks_hashes_response() {
        test_json(
            other::GET_ALT_BLOCKS_HASHES_RESPONSE,
            GetAltBlocksHashesResponse {
                base: AccessResponseBase::OK,
                blks_hashes: vec![Hex(hex!(
                    "8ee10db35b1baf943f201b303890a29e7d45437bd76c2bd4df0d2f2ee34be109"
                ))],
            },
        );
    }

    #[test]
    fn is_key_image_spent_request() {
        test_json(
            other::IS_KEY_IMAGE_SPENT_REQUEST,
            IsKeyImageSpentRequest {
                key_images: vec![
                    Hex(hex!(
                        "8d1bd8181bf7d857bdb281e0153d84cd55a3fcaa57c3e570f4a49f935850b5e3"
                    )),
                    Hex(hex!(
                        "7319134bfc50668251f5b899c66b005805ee255c136f0e1cecbb0f3a912e09d4"
                    )),
                ],
            },
        );
    }

    #[test]
    fn is_key_image_response() {
        test_json(
            other::IS_KEY_IMAGE_SPENT_RESPONSE,
            IsKeyImageSpentResponse {
                base: AccessResponseBase::OK,
                spent_status: vec![1, 1],
            },
        );
    }

    #[test]
    fn send_raw_transaction_request() {
        test_json(
            other::SEND_RAW_TRANSACTION_REQUEST,
            SendRawTransactionRequest {
                tx_as_hex: HexVec(
                    hex!("dc16fa8eaffe1484ca9014ea050e13131d3acf23b419f33bb4cc0b32b6c49308").into(),
                ),
                do_not_relay: false,
                do_sanity_checks: true,
            },
        );
    }

    #[test]
    fn send_raw_transaction_response() {
        test_json(
            other::SEND_RAW_TRANSACTION_RESPONSE,
            SendRawTransactionResponse {
                base: AccessResponseBase {
                    response_base: ResponseBase {
                        status: Status::Other("Failed".into()),
                        untrusted: false,
                    },
                    credits: 0,
                    top_hash: String::new(),
                },
                double_spend: false,
                fee_too_low: false,
                invalid_input: false,
                invalid_output: false,
                low_mixin: false,
                not_relayed: false,
                overspend: false,
                reason: String::new(),
                sanity_check_failed: false,
                too_big: false,
                too_few_outputs: false,
                tx_extra_too_big: false,
                nonzero_unlock_time: false,
            },
        );
    }

    #[test]
    fn start_mining_request() {
        test_json(other::START_MINING_REQUEST, StartMiningRequest {
            do_background_mining: false,
            ignore_battery: true,
            miner_address: "47xu3gQpF569au9C2ajo5SSMrWji6xnoE5vhr94EzFRaKAGw6hEGFXYAwVADKuRpzsjiU1PtmaVgcjUJF89ghGPhUXkndHc".into(),
            threads_count: 1
        });
    }

    #[test]
    fn start_mining_response() {
        test_json(
            other::START_MINING_RESPONSE,
            StartMiningResponse {
                base: ResponseBase::OK,
            },
        );
    }

    #[test]
    fn stop_mining_response() {
        test_json(
            other::STOP_MINING_RESPONSE,
            StopMiningResponse {
                base: ResponseBase::OK,
            },
        );
    }

    #[test]
    fn mining_status_response() {
        test_json(
            other::MINING_STATUS_RESPONSE,
            MiningStatusResponse {
                base: ResponseBase::OK,
                active: false,
                address: String::new(),
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
            },
        );
    }

    #[test]
    fn save_bc_response() {
        test_json(
            other::SAVE_BC_RESPONSE,
            SaveBcResponse {
                base: ResponseBase::OK,
            },
        );
    }

    #[test]
    fn get_peer_list_request() {
        test_json(
            other::GET_PEER_LIST_REQUEST,
            GetPeerListRequest {
                public_only: true,
                include_blocked: false,
            },
        );
    }

    #[test]
    fn get_peer_list_response() {
        test_json(
            other::GET_PEER_LIST_RESPONSE,
            GetPeerListResponse {
                base: ResponseBase::OK,
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
                    },
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
                    },
                ],
            },
        );
    }

    #[test]
    fn set_log_hash_rate_request() {
        test_json(
            other::SET_LOG_HASH_RATE_REQUEST,
            SetLogHashRateRequest { visible: true },
        );
    }

    #[test]
    fn set_log_hash_rate_response() {
        test_json(
            other::SET_LOG_HASH_RATE_RESPONSE,
            SetLogHashRateResponse {
                base: ResponseBase::OK,
            },
        );
    }

    #[test]
    fn set_log_level_request() {
        test_json(
            other::SET_LOG_LEVEL_REQUEST,
            SetLogLevelRequest { level: 1 },
        );
    }

    #[test]
    fn set_log_level_response() {
        test_json(
            other::SET_LOG_LEVEL_RESPONSE,
            SetLogLevelResponse {
                base: ResponseBase::OK,
            },
        );
    }

    #[test]
    fn set_log_categories_request() {
        test_json(
            other::SET_LOG_CATEGORIES_REQUEST,
            SetLogCategoriesRequest {
                categories: "*:INFO".into(),
            },
        );
    }

    #[test]
    fn set_log_categories_response() {
        test_json(
            other::SET_LOG_CATEGORIES_RESPONSE,
            SetLogCategoriesResponse {
                base: ResponseBase::OK,
                categories: "*:INFO".into(),
            },
        );
    }

    #[test]
    fn set_bootstrap_daemon_request() {
        test_json(
            other::SET_BOOTSTRAP_DAEMON_REQUEST,
            SetBootstrapDaemonRequest {
                address: "http://getmonero.org:18081".into(),
                username: String::new(),
                password: String::new(),
                proxy: String::new(),
            },
        );
    }

    #[test]
    fn set_bootstrap_daemon_response() {
        test_json(
            other::SET_BOOTSTRAP_DAEMON_RESPONSE,
            SetBootstrapDaemonResponse { status: Status::Ok },
        );
    }

    #[test]
    fn get_transaction_pool_stats_response() {
        test_json(
            other::GET_TRANSACTION_POOL_STATS_RESPONSE,
            GetTransactionPoolStatsResponse {
                base: AccessResponseBase::OK,
                pool_stats: TxpoolStats {
                    bytes_max: 11843,
                    bytes_med: 2219,
                    bytes_min: 1528,
                    bytes_total: 144192,
                    fee_total: 7018100000,
                    histo: vec![
                        TxpoolHisto {
                            bytes: 11219,
                            txs: 4,
                        },
                        TxpoolHisto {
                            bytes: 9737,
                            txs: 5,
                        },
                        TxpoolHisto {
                            bytes: 8757,
                            txs: 4,
                        },
                        TxpoolHisto {
                            bytes: 14763,
                            txs: 4,
                        },
                        TxpoolHisto {
                            bytes: 15007,
                            txs: 6,
                        },
                        TxpoolHisto {
                            bytes: 15924,
                            txs: 6,
                        },
                        TxpoolHisto {
                            bytes: 17869,
                            txs: 8,
                        },
                        TxpoolHisto {
                            bytes: 10894,
                            txs: 5,
                        },
                        TxpoolHisto {
                            bytes: 38485,
                            txs: 10,
                        },
                        TxpoolHisto {
                            bytes: 1537,
                            txs: 1,
                        },
                    ],
                    histo_98pc: 186,
                    num_10m: 0,
                    num_double_spends: 0,
                    num_failing: 0,
                    num_not_relayed: 0,
                    oldest: 1721261651,
                    txs_total: 53,
                },
            },
        );
    }

    #[test]
    fn stop_daemon_response() {
        test_json(
            other::STOP_DAEMON_RESPONSE,
            StopDaemonResponse { status: Status::Ok },
        );
    }

    #[test]
    fn get_limit_response() {
        test_json(
            other::GET_LIMIT_RESPONSE,
            GetLimitResponse {
                base: ResponseBase::OK,
                limit_down: 1280000,
                limit_up: 1280000,
            },
        );
    }

    #[test]
    fn set_limit_request() {
        test_json(
            other::SET_LIMIT_REQUEST,
            SetLimitRequest {
                limit_down: 1024,
                limit_up: 0,
            },
        );
    }

    #[test]
    fn set_limit_response() {
        test_json(
            other::SET_LIMIT_RESPONSE,
            SetLimitResponse {
                base: ResponseBase::OK,
                limit_down: 1024,
                limit_up: 128,
            },
        );
    }

    #[test]
    fn out_peers_request() {
        test_json(
            other::OUT_PEERS_REQUEST,
            OutPeersRequest {
                out_peers: 3232235535,
                set: true,
            },
        );
    }

    #[test]
    fn out_peers_response() {
        test_json(
            other::OUT_PEERS_RESPONSE,
            OutPeersResponse {
                base: ResponseBase::OK,
                out_peers: 3232235535,
            },
        );
    }

    #[test]
    fn get_net_stats_response() {
        test_json(
            other::GET_NET_STATS_RESPONSE,
            GetNetStatsResponse {
                base: ResponseBase::OK,
                start_time: 1721251858,
                total_bytes_in: 16283817214,
                total_bytes_out: 34225244079,
                total_packets_in: 5981922,
                total_packets_out: 3627107,
            },
        );
    }

    #[test]
    fn get_outs_request() {
        test_json(
            other::GET_OUTS_REQUEST,
            GetOutsRequest {
                outputs: vec![
                    GetOutputsOut {
                        amount: 1,
                        index: 0,
                    },
                    GetOutputsOut {
                        amount: 1,
                        index: 1,
                    },
                ],
                get_txid: true,
            },
        );
    }

    #[test]
    fn get_outs_response() {
        test_json(
            other::GET_OUTS_RESPONSE,
            GetOutsResponse {
                base: ResponseBase::OK,
                outs: vec![
                    OutKey {
                        height: 51941,
                        key: Hex(hex!(
                            "08980d939ec297dd597119f498ad69fed9ca55e3a68f29f2782aae887ef0cf8e"
                        )),
                        mask: Hex(hex!(
                            "1738eb7a677c6149228a2beaa21bea9e3370802d72a3eec790119580e02bd522"
                        )),
                        txid: HexVec(
                            hex!(
                                "9d651903b80fb70b9935b72081cd967f543662149aed3839222511acd9100601"
                            )
                            .into(),
                        ),
                        unlocked: true,
                    },
                    OutKey {
                        height: 51945,
                        key: Hex(hex!(
                            "454fe46c405be77625fa7e3389a04d3be392346983f27603561ac3a3a74f4a75"
                        )),
                        mask: Hex(hex!(
                            "1738eb7a677c6149228a2beaa21bea9e3370802d72a3eec790119580e02bd522"
                        )),
                        txid: HexVec(
                            hex!(
                                "230bff732dc5f225df14fff82aadd1bf11b3fb7ad3a03413c396a617e843f7d0"
                            )
                            .into(),
                        ),
                        unlocked: true,
                    },
                ],
            },
        );
    }

    #[test]
    fn update_request() {
        test_json(
            other::UPDATE_REQUEST,
            UpdateRequest {
                command: "check".into(),
                path: String::new(),
            },
        );
    }

    #[test]
    fn update_response() {
        test_json(
            other::UPDATE_RESPONSE,
            UpdateResponse {
                base: ResponseBase::OK,
                auto_uri: String::new(),
                hash: String::new(),
                path: String::new(),
                update: false,
                user_uri: String::new(),
                version: String::new(),
            },
        );
    }

    #[test]
    fn pop_blocks_request() {
        test_json(other::POP_BLOCKS_REQUEST, PopBlocksRequest { nblocks: 6 });
    }

    #[test]
    fn pop_blocks_response() {
        test_json(
            other::POP_BLOCKS_RESPONSE,
            PopBlocksResponse {
                base: ResponseBase::OK,
                height: 76482,
            },
        );
    }

    #[test]
    fn get_transaction_pool_hashes_response() {
        test_json(
            other::GET_TRANSACTION_POOL_HASHES_RESPONSE,
            GetTransactionPoolHashesResponse {
                base: ResponseBase::OK,
                tx_hashes: vec![
                    Hex(hex!(
                        "aa928aed888acd6152c60194d50a4df29b0b851be6169acf11b6a8e304dd6c03"
                    )),
                    Hex(hex!(
                        "794345f321a98f3135151f3056c0fdf8188646a8dab27de971428acf3551dd11"
                    )),
                    Hex(hex!(
                        "1e9d2ae11f2168a228942077483e70940d34e8658c972bbc3e7f7693b90edf17"
                    )),
                    Hex(hex!(
                        "7375c928f261d00f07197775eb0bfa756e5f23319819152faa0b3c670fe54c1b"
                    )),
                    Hex(hex!(
                        "2e4d5f8c5a45498f37fb8b6ca4ebc1efa0c371c38c901c77e66b08c072287329"
                    )),
                    Hex(hex!(
                        "eee6d596cf855adfb10e1597d2018e3a61897ac467ef1d4a5406b8d20bfbd52f"
                    )),
                    Hex(hex!(
                        "59c574d7ba9bb4558470f74503c7518946a85ea22c60fccfbdec108ce7d8f236"
                    )),
                    Hex(hex!(
                        "0d57bec1e1075a9e1ac45cf3b3ced1ad95ccdf2a50ce360190111282a0178655"
                    )),
                    Hex(hex!(
                        "60d627b2369714a40009c07d6185ebe7fa4af324fdfa8d95a37a936eb878d062"
                    )),
                    Hex(hex!(
                        "661d7e728a901a8cb4cf851447d9cd5752462687ed0b776b605ba706f06bdc7d"
                    )),
                    Hex(hex!(
                        "b80e1f09442b00b3fffe6db5d263be6267c7586620afff8112d5a8775a6fc58e"
                    )),
                    Hex(hex!(
                        "974063906d1ddfa914baf85176b0f689d616d23f3d71ed4798458c8b4f9b9d8f"
                    )),
                    Hex(hex!(
                        "d2575ae152a180be4981a9d2fc009afcd073adaa5c6d8b022c540a62d6c905bb"
                    )),
                    Hex(hex!(
                        "3d78aa80ee50f506683bab9f02855eb10257a08adceda7cbfbdfc26b10f6b1bb"
                    )),
                    Hex(hex!(
                        "8b5bc125bdb73b708500f734501d55088c5ac381a0879e1141634eaa72b6a4da"
                    )),
                    Hex(hex!(
                        "11c06f4d2f00c912ca07313ed2ea5366f3cae914a762bed258731d3d9e3706df"
                    )),
                    Hex(hex!(
                        "b3644dc7c9a3a53465fe80ad3769e516edaaeb7835e16fdd493aac110d472ae1"
                    )),
                    Hex(hex!(
                        "ed2478ad793b923dbf652c8612c40799d764e5468897021234a14a37346bc6ee"
                    )),
                ],
            },
        );
    }

    #[test]
    fn get_public_nodes_request() {
        test_json(
            other::GET_PUBLIC_NODES_REQUEST,
            GetPublicNodesRequest {
                gray: false,
                white: true,
                include_blocked: false,
            },
        );
    }

    #[test]
    fn get_publics_nodes_response() {
        test_json(
            other::GET_PUBLIC_NODES_RESPONSE,
            GetPublicNodesResponse {
                base: ResponseBase::OK,
                gray: vec![],
                white: vec![
                    PublicNode {
                        host: "70.52.75.3".into(),
                        last_seen: 1721246387,
                        rpc_credits_per_hash: 0,
                        rpc_port: 18081,
                    },
                    PublicNode {
                        host:
                            "zbjkbsxc5munw3qusl7j2hpcmikhqocdf4pqhnhtpzw5nt5jrmofptid.onion:18083"
                                .into(),
                        last_seen: 1720186288,
                        rpc_credits_per_hash: 0,
                        rpc_port: 18089,
                    },
                ],
            },
        );
    }
}

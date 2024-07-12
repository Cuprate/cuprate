//! Miscellaneous types.
//!
//! These are `struct`s that appear in request/response types.
//! For example, [`crate::json::GetConnectionsResponse`] contains
//! the [`crate::misc::ConnectionInfo`] struct defined here.

//---------------------------------------------------------------------------------------------------- Import
use std::fmt::Display;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    epee_object,
    macros::bytes::{Buf, BufMut},
    EpeeValue, Marker,
};

use crate::{
    constants::{
        CORE_RPC_STATUS_BUSY, CORE_RPC_STATUS_NOT_MINING, CORE_RPC_STATUS_OK,
        CORE_RPC_STATUS_PAYMENT_REQUIRED,
    },
    defaults::default_zero,
    macros::monero_definition_link,
};

//---------------------------------------------------------------------------------------------------- Macros
/// This macro (local to this file) defines all the misc types.
///
/// This macro:
/// 1. Defines a `pub struct` with all `pub` fields
/// 2. Implements `serde` on the struct
/// 3. Implements `epee` on the struct
///
/// When using, consider documenting:
/// - The original Monero definition site with [`monero_definition_link`]
/// - The request/responses where the `struct` is used
macro_rules! define_struct_and_impl_epee {
    (
        // Optional `struct` attributes.
        $( #[$struct_attr:meta] )*
        // The `struct`'s name.
        $struct_name:ident {
            // And any fields.
            $(
                $( #[$field_attr:meta] )* // Field attributes
                // Field name => the type => optional `epee_object` default value.
                $field_name:ident: $field_type:ty $(= $field_default:expr)?,
            )*
        }
    ) => {
        $( #[$struct_attr] )*
        #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        pub struct $struct_name {
            $(
                $( #[$field_attr] )*
                pub $field_name: $field_type,
            )*
        }

        #[cfg(feature = "epee")]
        epee_object! {
            $struct_name,
            $(
                $field_name: $field_type $(= $field_default)?,
            )*
        }
    };
}

//---------------------------------------------------------------------------------------------------- Type Definitions
define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        1163..=1212
    )]
    ///
    /// Used in:
    /// - [`crate::json::GetLastBlockHeaderResponse`]
    /// - [`crate::json::GetBlockHeaderByHashResponse`]
    /// - [`crate::json::GetBlockHeaderByHeightResponse`]
    /// - [`crate::json::GetBlockHeadersRangeResponse`]
    /// - [`crate::json::GetBlockResponse`]
    BlockHeader {
        block_size: u64,
        block_weight: u64,
        cumulative_difficulty_top64: u64,
        cumulative_difficulty: u64,
        depth: u64,
        difficulty_top64: u64,
        difficulty: u64,
        hash: String,
        height: u64,
        long_term_weight: u64,
        major_version: u8,
        miner_tx_hash: String,
        minor_version: u8,
        nonce: u32,
        num_txes: u64,
        orphan_status: bool,
        pow_hash: String,
        prev_hash: String,
        reward: u64,
        timestamp: u64,
        wide_cumulative_difficulty: String,
        wide_difficulty: String,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "cryptonote_protocol/cryptonote_protocol_defs.h",
        47..=116
    )]
    /// Used in [`crate::json::GetConnectionsResponse`].
    ConnectionInfo {
        address: String,
        address_type: u8,
        avg_download: u64,
        avg_upload: u64,
        connection_id: String,
        current_download: u64,
        current_upload: u64,
        height: u64,
        host: String,
        incoming: bool,
        ip: String,
        live_time: u64,
        localhost: bool,
        local_ip: bool,
        peer_id: String,
        port: String,
        pruning_seed: u32,
        recv_count: u64,
        recv_idle_time: u64,
        rpc_credits_per_hash: u32,
        rpc_port: u16,
        send_count: u64,
        send_idle_time: u64,
        ssl: bool,
        state: String,
        support_flags: u32,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        2034..=2047
    )]
    /// Used in [`crate::json::SetBansRequest`].
    SetBan {
        host: String,
        ip: u32,
        ban: bool,
        seconds: u32,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        1999..=2010
    )]
    /// Used in [`crate::json::GetBansResponse`].
    GetBan {
        host: String,
        ip: u32,
        seconds: u32,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        2139..=2156
    )]
    #[derive(Copy)]
    /// Used in [`crate::json::GetOutputHistogramResponse`].
    HistogramEntry {
        amount: u64,
        total_instances: u64,
        unlocked_instances: u64,
        recent_instances: u64,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        2180..=2191
    )]
    #[derive(Copy)]
    /// Used in [`crate::json::GetVersionResponse`].
    HardforkEntry {
        height: u64,
        hf_version: u8,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        2289..=2310
    )]
    /// Used in [`crate::json::GetAlternateChainsResponse`].
    ChainInfo {
        block_hash: String,
        block_hashes: Vec<String>,
        difficulty: u64,
        difficulty_top64: u64,
        height: u64,
        length: u64,
        main_chain_parent_block: String,
        wide_difficulty: String,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        2393..=2400
    )]
    /// Used in [`crate::json::SyncInfoResponse`].
    SyncInfoPeer {
        info: ConnectionInfo,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        2402..=2421
    )]
    /// Used in [`crate::json::SyncInfoResponse`].
    Span {
        connection_id: String,
        nblocks: u64,
        rate: u32,
        remote_address: String,
        size: u64,
        speed: u32,
        start_block_height: u64,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        1637..=1642
    )]
    #[derive(Copy)]
    /// Used in [`crate::json::GetTransactionPoolBacklogResponse`].
    TxBacklogEntry {
        weight: u64,
        fee: u64,
        time_in_pool: u64,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/rpc_handler.h",
        45..=50
    )]
    /// Used in [`crate::json::GetOutputDistributionResponse`].
    OutputDistributionData {
        distribution: Vec<u64>,
        start_height: u64,
        base: u64,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        1016..=1027
    )]
    /// Used in [`crate::json::GetMinerDataResponse`].
    ///
    /// Note that this is different than [`crate::misc::TxBacklogEntry`].
    GetMinerDataTxBacklogEntry {
        id: String,
        weight: u64,
        fee: u64,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        1070..=1079
    )]
    /// Used in [`crate::json::AddAuxPowRequest`].
    AuxPow {
        id: String,
        hash: String,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        192..=199
    )]
    /// Used in [`crate::bin::GetBlocksResponse`].
    TxOutputIndices {
        indices: Vec<u64>,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        201..=208
    )]
    /// Used in [`crate::bin::GetBlocksResponse`].
    BlockOutputIndices {
        indices: Vec<TxOutputIndices>,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        210..=221
    )]
    /// Used in [`crate::bin::GetBlocksResponse`].
    PoolTxInfo {
        tx_hash: [u8; 32],
        tx_blob: String,
        double_spend_seen: bool,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "cryptonote_protocol/cryptonote_protocol_defs.h",
        121..=131
    )]
    /// Used in [`crate::bin::GetBlocksResponse`].
    TxBlobEntry {
        blob: String,
        prunable_hash: [u8; 32],
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        512..=521
    )]
    #[derive(Copy)]
    ///
    /// Used in:
    /// - [`crate::bin::GetOutsRequest`]
    /// - [`crate::other::GetOutsRequest`]
    GetOutputsOut {
        amount: u64,
        index: u64,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        538..=553
    )]
    #[derive(Copy)]
    /// Used in [`crate::bin::GetOutsRequest`].
    OutKeyBin {
        key: [u8; 32],
        mask: [u8; 32],
        unlocked: bool,
        height: u64,
        txid: [u8; 32],
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        1335..=1367
    )]
    /// Used in [`crate::other::GetPeerListResponse`].
    Peer {
        id: u64,
        host: String,
        ip: u32,
        port: u16,
        #[cfg_attr(feature = "serde", serde(default = "default_zero"))]
        rpc_port: u16 = default_zero::<u16>(),
        #[cfg_attr(feature = "serde", serde(default = "default_zero"))]
        rpc_credits_per_hash: u32 = default_zero::<u32>(),
        last_seen: u64,
        #[cfg_attr(feature = "serde", serde(default = "default_zero"))]
        pruning_seed: u32 = default_zero::<u32>(),
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        1398..=1417
    )]
    ///
    /// Used in:
    /// - [`crate::other::GetPeerListResponse`]
    /// - [`crate::other::GetPublicNodesResponse`]
    PublicNode {
        host: String,
        last_seen: u64,
        rpc_port: u16,
        rpc_credits_per_hash: u32,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        1519..=1556
    )]
    /// Used in [`crate::other::GetTransactionPoolResponse`].
    TxInfo {
        blob_size: u64,
        do_not_relay: bool,
        double_spend_seen: bool,
        fee: u64,
        id_hash: String,
        kept_by_block: bool,
        last_failed_height: u64,
        last_failed_id_hash: String,
        last_relayed_time: u64,
        max_used_block_height: u64,
        max_used_block_id_hash: String,
        receive_time: u64,
        relayed: bool,
        tx_blob: String,
        tx_json: String, // TODO: this should be another struct
        #[cfg_attr(feature = "serde", serde(default = "default_zero"))]
        weight: u64 = default_zero::<u64>(),
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        1558..=1567
    )]
    /// Used in [`crate::other::GetTransactionPoolResponse`].
    SpentKeyImageInfo {
        id_hash: String,
        txs_hashes: Vec<String>,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        1666..=1675
    )]
    #[derive(Copy)]
    /// Used in [`crate::other::GetTransactionPoolStatsResponse`].
    TxpoolHisto {
        txs: u32,
        bytes: u64,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        1677..=1710
    )]
    /// Used in [`crate::other::GetTransactionPoolStatsResponse`].
    TxpoolStats {
        bytes_max: u32,
        bytes_med: u32,
        bytes_min: u32,
        bytes_total: u64,
        fee_total: u64,
        histo_98pc: u64,
        histo: Vec<TxpoolHisto>,
        num_10m: u32,
        num_double_spends: u32,
        num_failing: u32,
        num_not_relayed: u32,
        oldest: u64,
        txs_total: u32,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        582..=597
    )]
    /// Used in [`crate::other::GetOutsResponse`].
    OutKey {
        key: String,
        mask: String,
        unlocked: bool,
        height: u64,
        txid: String,
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}

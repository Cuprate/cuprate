//! Miscellaneous types.
//!
//! These are `struct`s that appear in request/response types.
//! For example, [`crate::json::GetConnectionsResponse`] contains
//! the [`crate::misc::ConnectionInfo`] struct defined here.

//---------------------------------------------------------------------------------------------------- Import
use cuprate_hex::Hex;
use cuprate_types::HardFork;

#[cfg(any(feature = "epee", feature = "serde"))]
use crate::defaults::default_zero;

use crate::macros::monero_definition_link;

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
    ($(
        // Optional `struct` attributes.
        $( #[$struct_attr:meta] )*
        // The `struct`'s name.
        $struct_name:ident {
            // And any fields.
            $(
                $( #[$field_attr:meta] )* // Field attributes
                // Field name => the type => optional `epee_object` default value.
                $field_name:ident: $field_type:ty $(= $field_default:expr_2021)?,
            )*
        }
    )*) => {
        $(
            #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
            $( #[$struct_attr] )*
            pub struct $struct_name {
                $(
                    $( #[$field_attr] )*
                    pub $field_name: $field_type,
                )*
            }

            #[cfg(feature = "epee")]
            cuprate_epee_encoding::epee_object! {
                $struct_name,
                $(
                    $field_name: $field_type $(= $field_default)?,
                )*
            }
        )*
    };
}

//---------------------------------------------------------------------------------------------------- Type Definitions
define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
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
        hash: Hex<32>,
        height: u64,
        long_term_weight: u64,
        major_version: HardFork,
        miner_tx_hash: Hex<32>,
        minor_version: u8,
        nonce: u32,
        num_txes: u64,
        orphan_status: bool,
        /// This is an empty string if the `fill_pow_hash` param is `false`.
        pow_hash: String,
        prev_hash: Hex<32>,
        reward: u64,
        timestamp: u64,
        wide_cumulative_difficulty: String,
        wide_difficulty: String,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "cryptonote_protocol/cryptonote_protocol_defs.h",
        47..=116
    )]
    /// Used in [`crate::json::GetConnectionsResponse`].
    ConnectionInfo {
        address: String,
        address_type: cuprate_types::AddressType,
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
        // Exists in the original definition, but isn't
        // used or (de)serialized for RPC purposes.
        // ssl: bool,
        state: cuprate_types::ConnectionState,
        support_flags: u32,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        2034..=2047
    )]
    /// Used in [`crate::json::SetBansRequest`].
    SetBan {
        #[cfg_attr(feature = "serde", serde(default = "crate::defaults::default_string"))]
        host: String,
        #[cfg_attr(feature = "serde", serde(default = "default_zero"))]
        ip: u32,
        ban: bool,
        seconds: u32,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
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
        "cc73fe71162d564ffda8e549b79a350bca53c454",
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
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        2289..=2310
    )]
    /// Used in [`crate::json::GetAlternateChainsResponse`].
    ChainInfo {
        block_hash: Hex<32>,
        block_hashes: Vec<Hex<32>>,
        difficulty: u64,
        difficulty_top64: u64,
        height: u64,
        length: u64,
        main_chain_parent_block: Hex<32>,
        wide_difficulty: String,
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
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
        "cc73fe71162d564ffda8e549b79a350bca53c454",
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
        "cc73fe71162d564ffda8e549b79a350bca53c454",
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
        "cc73fe71162d564ffda8e549b79a350bca53c454",
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
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        1519..=1556
    )]
    /// Used in [`crate::other::GetTransactionPoolResponse`].
    TxInfo {
        blob_size: u64,
        do_not_relay: bool,
        double_spend_seen: bool,
        fee: u64,
        id_hash: Hex<32>,
        kept_by_block: bool,
        last_failed_height: u64,
        last_failed_id_hash: Hex<32>,
        last_relayed_time: u64,
        max_used_block_height: u64,
        max_used_block_id_hash: Hex<32>,
        receive_time: u64,
        relayed: bool,
        tx_blob: String,
        tx_json: cuprate_types::json::tx::Transaction,
        #[cfg_attr(feature = "serde", serde(default = "default_zero"))]
        weight: u64 = default_zero::<u64>(),
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        1558..=1567
    )]
    /// Used in [`crate::other::GetTransactionPoolResponse`].
    SpentKeyImageInfo {
        id_hash: Hex<32>,
        txs_hashes: Vec<Hex<32>>,
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}

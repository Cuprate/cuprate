//! TODO

//---------------------------------------------------------------------------------------------------- Lints
#![allow(
    missing_docs, // Docs are at: <https://www.getmonero.org/resources/developer-guides/daemon-rpc.html>
    clippy::struct_excessive_bools, // hey man, tell that to the people who wrote `monerod`
)]

//---------------------------------------------------------------------------------------------------- Import
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::epee_object;

use crate::macros::monero_definition_link;

//---------------------------------------------------------------------------------------------------- Macros
/// TODO
macro_rules! define_struct_and_impl_epee {
    (
        $( #[$struct_attr:meta] )*
        $struct_name:ident {
            // And any fields.
            $(
                $( #[$field_attr:meta] )*
                $field_name:ident: $field_type:ty,
            )*
        }
    ) => {
        $( #[$struct_attr] )*
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
                $field_name: $field_type,
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
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    /// Used in [`crate::json::GetOutputHistogramResponse`].
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    /// Used in [`crate::json::GetVersionResponse`].
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    Peer {
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
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    /// Used in [`crate::json::GetTransactionPoolBacklog`].
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    /// Used in [`crate::json::GetOutputDistribution`].
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    /// Note that this is different than [`crate::misc::TxbacklogEntry`].
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    /// Used in [`crate::json::GetAuxPowRequest`].
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    AuxPow {
        id: String,
        hash: String,
    }
}

//---------------------------------------------------------------------------------------------------- Custom serde
// This section is for `struct`s that have custom (de)serialization code.

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

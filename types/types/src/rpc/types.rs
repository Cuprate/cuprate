//! Various types (in)directly used in RPC.

use cuprate_fixed_bytes::ByteArrayVec;
use cuprate_hex::Hex;

use crate::{AddressType, ConnectionState, HardFork};

const fn default_string() -> String {
    String::new()
}

fn default_zero<T: From<u8>>() -> T {
    T::from(0)
}

/// Output a string link to `monerod` source code.
macro_rules! monero_definition_link {
    (
        $commit:literal, // Git commit hash
        $file_path:literal, // File path within `monerod`'s `src/`, e.g. `rpc/core_rpc_server_commands_defs.h`
        $start:literal$(..=$end:literal)? // File lines, e.g. `0..=123` or `0`
    ) => {
        concat!(
            "[Definition](https://github.com/monero-project/monero/blob/",
            $commit,
            "/src/",
            $file_path,
            "#L",
            stringify!($start),
            $(
                "-L",
                stringify!($end),
            )?
            ")."
        )
    };
}

/// This macro (local to this file) defines all the misc types.
///
/// This macro:
/// 1. Defines a `struct` with all `pub` fields
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

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        1163..=1212
    )]
    BlockHeader {
        block_weight: u64,
        cumulative_difficulty_top64: u64,
        cumulative_difficulty: u64,
        depth: u64,
        difficulty_top64: u64,
        difficulty: u64,
        hash: [u8; 32],
        height: u64,
        long_term_weight: u64,
        major_version: HardFork,
        miner_tx_hash: [u8; 32],
        minor_version: u8,
        nonce: u32,
        num_txes: u64,
        orphan_status: bool,
        /// This is [`None`] if the `fill_pow_hash` param is `false`.
        pow_hash: Option<[u8; 32]>,
        prev_hash: [u8; 32],
        reward: u64,
        timestamp: u64,
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "cryptonote_protocol/cryptonote_protocol_defs.h",
        47..=116
    )]
    ConnectionInfo {
        address: String,
        address_type: AddressType,
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
        state: ConnectionState,
        support_flags: u32,
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        2034..=2047
    )]
    SetBan {
        #[cfg_attr(feature = "serde", serde(default = "default_string"))]
        host: String,
        #[cfg_attr(feature = "serde", serde(default = "default_zero"))]
        ip: u32,
        ban: bool,
        seconds: u32,
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        1999..=2010
    )]
    GetBan {
        host: String,
        ip: u32,
        seconds: u32,
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        2139..=2156
    )]
    #[derive(Copy)]
    HistogramEntry {
        amount: u64,
        total_instances: u64,
        unlocked_instances: u64,
        recent_instances: u64,
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        2180..=2191
    )]
    #[derive(Copy)]
    HardForkEntry {
        height: u64,
        hf_version: HardFork,
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        2289..=2310
    )]
    ChainInfo {
        block_hash: [u8; 32],
        block_hashes: Vec<[u8; 32]>,
        difficulty_top64: u64,
        difficulty: u64,
        height: u64,
        length: u64,
        main_chain_parent_block: [u8; 32],
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        2393..=2400
    )]
    SyncInfoPeer {
        info: ConnectionInfo,
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        2402..=2421
    )]
    Span {
        connection_id: String,
        nblocks: u64,
        rate: u32,
        remote_address: String,
        size: u64,
        speed: u32,
        start_block_height: u64,
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        1637..=1642
    )]
    #[derive(Copy)]
    TxBacklogEntry {
        weight: u64,
        fee: u64,
        time_in_pool: u64,
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/rpc_handler.h",
        45..=50
    )]
    OutputDistributionData {
        distribution: Vec<u64>,
        start_height: u64,
        base: u64,
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        1016..=1027
    )]
    GetMinerDataTxBacklogEntry {
        id: Hex<32>,
        weight: u64,
        fee: u64,
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        1070..=1079
    )]
    AuxPow {
        id: Hex<32>,
        hash: Hex<32>,
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        192..=199
    )]
    TxOutputIndices {
        indices: Vec<u64>,
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        201..=208
    )]
    BlockOutputIndices {
        indices: Vec<TxOutputIndices>,
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        512..=521
    )]
    #[derive(Copy)]
    GetOutputsOut {
        amount: u64,
        index: u64,
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        538..=553
    )]
    OutKeyBin {
        key: [u8; 32],
        mask: [u8; 32],
        unlocked: bool,
        height: u64,
        txid: [u8; 32],
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        1335..=1367
    )]
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

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        1398..=1417
    )]
    PublicNode {
        host: String,
        last_seen: u64,
        rpc_port: u16,
        rpc_credits_per_hash: u32,
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        1519..=1556
    )]
    TxInfo {
        blob_size: u64,
        do_not_relay: bool,
        double_spend_seen: bool,
        fee: u64,
        id_hash: [u8; 32],
        kept_by_block: bool,
        last_failed_height: u64,
        last_failed_id_hash: [u8; 32],
        last_relayed_time: u64,
        max_used_block_height: u64,
        max_used_block_id_hash: [u8; 32],
        receive_time: u64,
        relayed: bool,
        tx_blob: Vec<u8>,
        tx_json: crate::json::tx::Transaction,
        #[cfg_attr(feature = "serde", serde(default = "default_zero"))]
        weight: u64 = default_zero::<u64>(),
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        1558..=1567
    )]
    SpentKeyImageInfo {
        id_hash: [u8; 32],
        txs_hashes: Vec<[u8; 32]>,
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        1666..=1675
    )]
    #[derive(Copy)]
    TxpoolHisto {
        txs: u32,
        bytes: u64,
    }

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        1677..=1710
    )]
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

    #[doc = monero_definition_link!(
        "cc73fe71162d564ffda8e549b79a350bca53c454",
        "rpc/core_rpc_server_commands_defs.h",
        582..=597
    )]
    OutKey {
        key: Hex<32>,
        mask: Hex<32>,
        unlocked: bool,
        height: u64,
        txid: Hex<32>,
    }

    #[doc = monero_definition_link!(
        "893916ad091a92e765ce3241b94e706ad012b62a",
        "blockchain_db/lmdb/db_lmdb.cpp",
        4222
    )]
    OutputHistogramInput {
        amounts: Vec<u64>,
        min_count: u64,
        max_count: u64,
        unlocked: bool,
        recent_cutoff: u64,
    }

    #[doc = monero_definition_link!(
        "893916ad091a92e765ce3241b94e706ad012b62a",
        "rpc/core_rpc_server_commands_defs.h",
        2139..=2156
    )]
    OutputHistogramEntry {
        amount: u64,
        total_instances: u64,
        unlocked_instances: u64,
        recent_instances: u64,
    }

    #[doc = monero_definition_link!(
        "893916ad091a92e765ce3241b94e706ad012b62a",
        "rpc/core_rpc_server_commands_defs.h",
        2228..=2247
    )]
    CoinbaseTxSum {
        emission_amount_top64: u64,
        emission_amount: u64,
        fee_amount_top64: u64,
        fee_amount: u64,
    }

    #[doc = monero_definition_link!(
        "893916ad091a92e765ce3241b94e706ad012b62a",
        "rpc/core_rpc_server_commands_defs.h",
        1027..=1033
    )]
    MinerData {
        major_version: u8,
        height: u64,
        prev_id: [u8; 32],
        seed_hash: [u8; 32],
        difficulty_top64: u64,
        difficulty: u64,
        median_weight: u64,
        already_generated_coins: u64,
        tx_backlog: Vec<MinerDataTxBacklogEntry>,
    }

    #[doc = monero_definition_link!(
        "893916ad091a92e765ce3241b94e706ad012b62a",
        "rpc/core_rpc_server_commands_defs.h",
        1037..=1039
    )]
    MinerDataTxBacklogEntry {
        id: [u8; 32],
        weight: u64,
        fee: u64,
    }

    #[doc = monero_definition_link!(
        "893916ad091a92e765ce3241b94e706ad012b62a",
        "rpc/core_rpc_server_commands_defs.h",
        1973..=1980
    )]
    HardForkInfo {
        earliest_height: u64,
        enabled: bool,
        state: u32,
        threshold: u32,
        version: u8,
        votes: u32,
        voting: u8,
        window: u32,
    }

    #[doc = monero_definition_link!(
        "893916ad091a92e765ce3241b94e706ad012b62a",
        "rpc/core_rpc_server_commands_defs.h",
        2264..=2267
    )]
    FeeEstimate {
        fee: u64,
        fees: Vec<u64>,
        quantization_mask: u64,
    }

    #[doc = monero_definition_link!(
        "893916ad091a92e765ce3241b94e706ad012b62a",
        "rpc/core_rpc_server_commands_defs.h",
        1115..=1119
    )]
    AddAuxPow {
        blocktemplate_blob: Vec<u8>,
        blockhashing_blob: Vec<u8>,
        merkle_root: [u8; 32],
        merkle_tree_depth: u64,
        aux_pow: Vec<AuxPow>,
    }

    #[doc = monero_definition_link!(
        "893916ad091a92e765ce3241b94e706ad012b62a",
        "rpc/core_rpc_server_commands_defs.h",
        227..=229
    )]
    PoolTxInfo {
        tx_hash: [u8; 32],
        tx_blob: Vec<u8>,
        double_spend_seen: bool,
    }

    #[doc = monero_definition_link!(
        "893916ad091a92e765ce3241b94e706ad012b62a",
        "rpc/core_rpc_server_commands_defs.h",
        254..=256
    )]
    PoolInfoIncremental {
        added_pool_txs: Vec<PoolTxInfo>,
        remaining_added_pool_txids: ByteArrayVec<32>,
        removed_pool_txids: ByteArrayVec<32>,
    }

    #[doc = monero_definition_link!(
        "893916ad091a92e765ce3241b94e706ad012b62a",
        "rpc/core_rpc_server_commands_defs.h",
        254..=256
    )]
    PoolInfoFull {
        added_pool_txs: Vec<PoolTxInfo>,
        remaining_added_pool_txids: ByteArrayVec<32>,
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

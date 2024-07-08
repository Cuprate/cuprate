//! Miscellaneous types.
//!
//! These are `struct`s that appear in request/response types.
//! For example, [`crate::json::GetConnectionsResponse`] contains
//! the [`crate::misc::ConnectionInfo`] struct defined here.

//---------------------------------------------------------------------------------------------------- Lints
#![allow(
    missing_docs, // Docs are at: <https://www.getmonero.org/resources/developer-guides/daemon-rpc.html>
    clippy::struct_excessive_bools, // hey man, tell that to the people who wrote `monerod`
)]

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
        CORE_RPC_STATUS_PAYMENT_REQUIRED, CORE_RPC_STATUS_UNKNOWN,
    },
    defaults::{default_u16, default_u32, default_u64},
    macros::monero_definition_link,
};

//---------------------------------------------------------------------------------------------------- Macros
/// This macro (local to this file) defines all the misc types.
///
/// This macro:
/// 1. Defines a `pub struct` with all `pub` fields
/// 2. Implements `epee` on the struct
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
    /// Used in [`crate::bin::GetOutsRequest`].
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
    OutKey {
        key: u8, // TODO: crypto::public_key,
        mask: u8, // TODO: rct::key,
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
        #[cfg_attr(feature = "serde", serde(default = "default_u16"))]
        rpc_port: u16 = default_u16(),
        #[cfg_attr(feature = "serde", serde(default = "default_u32"))]
        rpc_credits_per_hash: u32 = default_u32(),
        last_seen: u64,
        #[cfg_attr(feature = "serde", serde(default = "default_u32"))]
        pruning_seed: u32 = default_u32(),
    }
}

define_struct_and_impl_epee! {
    #[doc = monero_definition_link!(
        cc73fe71162d564ffda8e549b79a350bca53c454,
        "rpc/core_rpc_server_commands_defs.h",
        1398..=1417
    )]
    /// Used in [`crate::other::GetPeerListResponse`].
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
        id_hash: String,
        tx_json: String,
        blob_size: u64,
        #[cfg_attr(feature = "serde", serde(default = "default_u64"))]
        weight: u64 = default_u64(),
        fee: u64,
        max_used_block_id_hash: String,
        max_used_block_height: u64,
        kept_by_block: bool,
        last_failed_height: u64,
        last_failed_id_hash: String,
        receive_time: u64,
        relayed: bool,
        last_relayed_time: u64,
        do_not_relay: bool,
        double_spend_seen: bool,
        tx_blob: String,
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

//---------------------------------------------------------------------------------------------------- TODO
// TODO - weird types.

#[doc = monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
    "rpc/core_rpc_server_commands_defs.h",
    210..=221
)]
/// Used in [`crate::bin::GetBlocksResponse`].
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BlockCompleteEntry {
    pub pruned: bool,
    pub block: String,
    pub block_weight: u64,
    pub txs: Vec<TxBlobEntry>,
}

// TODO: custom epee
// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_defs.h#L138-L163>
#[cfg(feature = "epee")]
epee_object! {
    BlockCompleteEntry,
    pruned: bool,
    block: String,
    block_weight: u64,
    txs: Vec<TxBlobEntry>,
}

/// Used in [`crate::bin::GetBlocksResponse`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(u8)]
pub enum PoolInfoExtent {
    None = 0,
    Incremental = 1,
    Full = 2,
}

// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_defs.h#L138-L163>
#[cfg(feature = "epee")]
impl EpeeValue for PoolInfoExtent {
    const MARKER: Marker = <String as EpeeValue>::MARKER;

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> cuprate_epee_encoding::Result<Self> {
        todo!()
    }

    fn should_write(&self) -> bool {
        todo!()
    }

    fn epee_default_value() -> Option<Self> {
        todo!()
    }

    fn write<B: BufMut>(self, w: &mut B) -> cuprate_epee_encoding::Result<()> {
        todo!()
    }
}

#[doc = monero_definition_link!(
    cc73fe71162d564ffda8e549b79a350bca53c454,
    "rpc/core_rpc_server_commands_defs.h",
    389..=428
)]
/// Used in [`crate::other::GetTransactionsResponse`].
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TxEntry {
    pub as_hex: String,
    pub as_json: String,
    pub block_height: u64,
    pub block_timestamp: u64,
    pub confirmations: u64,
    pub double_spend_seen: bool,
    pub in_pool: bool,
    pub output_indices: Vec<u64>,
    pub prunable_as_hex: String,
    pub prunable_hash: String,
    pub pruned_as_hex: String,
    pub received_timestamp: u64,
    pub relayed: bool,
    pub tx_hash: String,
}

// TODO: custom epee
// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_defs.h#L406-427>
#[cfg(feature = "epee")]
epee_object! {
    TxEntry,
    as_hex: String,
    as_json: String, // TODO: should be its own struct
    block_height: u64,
    block_timestamp: u64,
    confirmations: u64,
    double_spend_seen: bool,
    in_pool: bool,
    output_indices: Vec<u64>,
    prunable_as_hex: String,
    prunable_hash: String,
    pruned_as_hex: String,
    received_timestamp: u64,
    relayed: bool,
    tx_hash: String,
}

//---------------------------------------------------------------------------------------------------- TODO
/// Used in [`crate::other::IsKeyImageSpentResponse`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[repr(u8)]
pub enum KeyImageSpentStatus {
    Unspent = 0,
    SpentInBlockchain = 1,
    SpentInPool = 2,
}

#[cfg(feature = "epee")]
impl EpeeValue for KeyImageSpentStatus {
    const MARKER: Marker = <String as EpeeValue>::MARKER;

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> cuprate_epee_encoding::Result<Self> {
        todo!()
    }

    fn should_write(&self) -> bool {
        todo!()
    }

    fn epee_default_value() -> Option<Self> {
        todo!()
    }

    fn write<B: BufMut>(self, w: &mut B) -> cuprate_epee_encoding::Result<()> {
        todo!()
    }
}

//---------------------------------------------------------------------------------------------------- Status
/// RPC response status.
///
/// This type represents `monerod`'s frequently appearing string field, `status`.
///
/// This field appears within RPC [JSON response](crate::json) types.
///
/// Reference: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L78-L81>.
///
/// ## Serialization and string formatting
/// ```rust
/// use cuprate_rpc_types::{
///     misc::Status,
///     CORE_RPC_STATUS_BUSY, CORE_RPC_STATUS_NOT_MINING, CORE_RPC_STATUS_OK,
///     CORE_RPC_STATUS_PAYMENT_REQUIRED, CORE_RPC_STATUS_UNKNOWN
/// };
/// use serde_json::to_string;
///
/// let unknown = Status::Unknown;
///
/// assert_eq!(to_string(&Status::Ok).unwrap(),              r#""OK""#);
/// assert_eq!(to_string(&Status::Busy).unwrap(),            r#""BUSY""#);
/// assert_eq!(to_string(&Status::NotMining).unwrap(),       r#""NOT MINING""#);
/// assert_eq!(to_string(&Status::PaymentRequired).unwrap(), r#""PAYMENT REQUIRED""#);
/// assert_eq!(to_string(&unknown).unwrap(),                 r#""UNKNOWN""#);
///
/// assert_eq!(Status::Ok.as_ref(),              CORE_RPC_STATUS_OK);
/// assert_eq!(Status::Busy.as_ref(),            CORE_RPC_STATUS_BUSY);
/// assert_eq!(Status::NotMining.as_ref(),       CORE_RPC_STATUS_NOT_MINING);
/// assert_eq!(Status::PaymentRequired.as_ref(), CORE_RPC_STATUS_PAYMENT_REQUIRED);
/// assert_eq!(unknown.as_ref(),                 CORE_RPC_STATUS_UNKNOWN);
///
/// assert_eq!(format!("{}", Status::Ok),              CORE_RPC_STATUS_OK);
/// assert_eq!(format!("{}", Status::Busy),            CORE_RPC_STATUS_BUSY);
/// assert_eq!(format!("{}", Status::NotMining),       CORE_RPC_STATUS_NOT_MINING);
/// assert_eq!(format!("{}", Status::PaymentRequired), CORE_RPC_STATUS_PAYMENT_REQUIRED);
/// assert_eq!(format!("{}", unknown),                 CORE_RPC_STATUS_UNKNOWN);
///
/// assert_eq!(format!("{:?}", Status::Ok),              "Ok");
/// assert_eq!(format!("{:?}", Status::Busy),            "Busy");
/// assert_eq!(format!("{:?}", Status::NotMining),       "NotMining");
/// assert_eq!(format!("{:?}", Status::PaymentRequired), "PaymentRequired");
/// assert_eq!(format!("{:?}", unknown),                 "Unknown");
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Status {
    // FIXME:
    // `#[serde(rename = "")]` only takes raw string literals?
    // We have to re-type the constants here...
    /// Successful RPC response, everything is OK; [`CORE_RPC_STATUS_OK`].
    #[cfg_attr(feature = "serde", serde(rename = "OK"))]
    #[default]
    Ok,

    /// The daemon is busy, try later; [`CORE_RPC_STATUS_BUSY`].
    #[cfg_attr(feature = "serde", serde(rename = "BUSY"))]
    Busy,

    /// The daemon is not mining; [`CORE_RPC_STATUS_NOT_MINING`].
    #[cfg_attr(feature = "serde", serde(rename = "NOT MINING"))]
    NotMining,

    /// Payment is required for RPC; [`CORE_RPC_STATUS_PAYMENT_REQUIRED`].
    #[cfg_attr(feature = "serde", serde(rename = "PAYMENT REQUIRED"))]
    PaymentRequired,

    /// Some unknown other string; [`CORE_RPC_STATUS_UNKNOWN`].
    ///
    /// This exists to act as a catch-all if `monerod` adds
    /// a string and a Cuprate node hasn't updated yet.
    ///
    /// The reason this isn't `Unknown(String)` is because that
    /// disallows [`Status`] to be [`Copy`], and thus other types
    /// that contain it.
    #[cfg_attr(feature = "serde", serde(other))]
    #[cfg_attr(feature = "serde", serde(rename = "UNKNOWN"))]
    Unknown,
}

impl From<String> for Status {
    fn from(s: String) -> Self {
        match s.as_str() {
            CORE_RPC_STATUS_OK => Self::Ok,
            CORE_RPC_STATUS_BUSY => Self::Busy,
            CORE_RPC_STATUS_NOT_MINING => Self::NotMining,
            CORE_RPC_STATUS_PAYMENT_REQUIRED => Self::PaymentRequired,
            _ => Self::Unknown,
        }
    }
}

impl AsRef<str> for Status {
    fn as_ref(&self) -> &str {
        match self {
            Self::Ok => CORE_RPC_STATUS_OK,
            Self::Busy => CORE_RPC_STATUS_BUSY,
            Self::NotMining => CORE_RPC_STATUS_NOT_MINING,
            Self::PaymentRequired => CORE_RPC_STATUS_PAYMENT_REQUIRED,
            Self::Unknown => CORE_RPC_STATUS_UNKNOWN,
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

// [`Status`] is essentially a [`String`] when it comes to
// (de)serialization, except when writing we usually have
// access to a `&'static str` and don't need to allocate.
//
// See below for more impl info:
// <https://github.com/Cuprate/cuprate/blob/bef2a2cbd4e1194991751d1fbc96603cba8c7a51/net/epee-encoding/src/value.rs#L366-L392>.
#[cfg(feature = "epee")]
impl EpeeValue for Status {
    const MARKER: Marker = <String as EpeeValue>::MARKER;

    fn read<B: Buf>(r: &mut B, marker: &Marker) -> cuprate_epee_encoding::Result<Self> {
        let string = <String as EpeeValue>::read(r, marker)?;
        Ok(Self::from(string))
    }

    fn should_write(&self) -> bool {
        true
    }

    fn epee_default_value() -> Option<Self> {
        // <https://github.com/Cuprate/cuprate/pull/147#discussion_r1654992559>
        Some(Self::Unknown)
    }

    fn write<B: BufMut>(self, w: &mut B) -> cuprate_epee_encoding::Result<()> {
        cuprate_epee_encoding::write_bytes(self.as_ref(), w)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use super::*;

    // Test epee (de)serialization works.
    #[test]
    #[cfg(feature = "epee")]
    fn epee() {
        for status in [
            Status::Ok,
            Status::Busy,
            Status::NotMining,
            Status::PaymentRequired,
            Status::Unknown,
        ] {
            let mut buf = vec![];

            <Status as EpeeValue>::write(status, &mut buf).unwrap();
            let status2 =
                <Status as EpeeValue>::read(&mut buf.as_slice(), &<Status as EpeeValue>::MARKER)
                    .unwrap();

            assert_eq!(status, status2);
        }
    }
}

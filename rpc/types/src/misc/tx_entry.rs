//! TODO

//---------------------------------------------------------------------------------------------------- Use
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{
    epee_object,
    macros::bytes::{Buf, BufMut},
    EpeeValue, Marker,
};

//---------------------------------------------------------------------------------------------------- TxEntry
#[doc = crate::macros::monero_definition_link!(
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
// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L406-L427>
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

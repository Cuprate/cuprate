//! TODO

//---------------------------------------------------------------------------------------------------- Use
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::epee_object;

use crate::misc::TxBlobEntry;

//---------------------------------------------------------------------------------------------------- BlockCompleteEntry
#[doc = crate::macros::monero_definition_link!(
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

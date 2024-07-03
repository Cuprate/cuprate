//! TODO

//---------------------------------------------------------------------------------------------------- Import
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "epee")]
use cuprate_epee_encoding::epee_object;

use crate::macros::monero_definition_link;

//---------------------------------------------------------------------------------------------------- BlockHeader
#[doc = monero_definition_link!(cc73fe71162d564ffda8e549b79a350bca53c454 => core_rpc_server_commands_defs.h => 1163..=1212)]
/// TODO.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[allow(missing_docs)]
pub struct BlockHeader {
    pub major_version: u8,
    pub minor_version: u8,
    pub timestamp: u64,
    pub prev_hash: String,
    pub nonce: u32,
    pub orphan_status: bool,
    pub height: u64,
    pub depth: u64,
    pub hash: String,
    pub difficulty: u64,
    pub wide_difficulty: String,
    pub difficulty_top64: u64,
    pub cumulative_difficulty: u64,
    pub wide_cumulative_difficulty: String,
    pub cumulative_difficulty_top64: u64,
    pub reward: u64,
    pub block_size: u64,
    pub block_weight: u64,
    pub num_txes: u64,
    pub pow_hash: String,
    pub long_term_weight: u64,
    pub miner_tx_hash: String,
}

#[cfg(feature = "epee")]
epee_object! {
    BlockHeader,
    major_version: u8,
    minor_version: u8,
    timestamp: u64,
    prev_hash: String,
    nonce: u32,
    orphan_status: bool,
    height: u64,
    depth: u64,
    hash: String,
    difficulty: u64,
    wide_difficulty: String,
    difficulty_top64: u64,
    cumulative_difficulty: u64,
    wide_cumulative_difficulty: String,
    cumulative_difficulty_top64: u64,
    reward: u64,
    block_size: u64,
    block_weight: u64,
    num_txes: u64,
    pow_hash: String,
    long_term_weight: u64,
    miner_tx_hash: String,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

// Rust Levin Library
// Written in 2023 by
//   Cuprate Contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//

//! Common types that are used across multiple messages.
//
use epee_encoding::EpeeObject;

use crate::NetworkAddress;

mod builders;

#[derive(Debug, Clone, Copy, EpeeObject, PartialEq, Eq)]
pub struct PeerSupportFlags {
    #[epee_default(0)]
    pub support_flags: u32,
}
/*
impl PeerSupportFlags {
    const FLUFFY_BLOCKS: u32 = 0b0000_0001;
    /// checks if `self` has all the flags that `other` has
    pub fn contains(&self, other: &PeerSupportFlags) -> bool {
        self.0. & other.0 == other.0
    }
    pub fn supports_fluffy_blocks(&self) -> bool {
        self.0 & Self::FLUFFY_BLOCKS == Self::FLUFFY_BLOCKS
    }
    pub fn get_support_flag_fluffy_blocks() -> Self {
        PeerSupportFlags {
            support_flags: Self::FLUFFY_BLOCKS,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}
*/
impl From<u8> for PeerSupportFlags {
    fn from(value: u8) -> Self {
        PeerSupportFlags {
            support_flags: value.into(),
        }
    }
}

impl From<u32> for PeerSupportFlags {
    fn from(support_flags: u32) -> Self {
        PeerSupportFlags { support_flags }
    }
}

/// Basic Node Data, information on the connected peer
#[derive(Debug, Clone, EpeeObject, PartialEq, Eq)]
pub struct BasicNodeData {
    /// Port
    pub my_port: u32,
    /// The Network Id
    pub network_id: [u8; 16],
    /// Peer ID
    pub peer_id: u64,
    /// The Peers Support Flags
    /// (If this is not in the message the default is 0)
    #[epee_flatten]
    pub support_flags: PeerSupportFlags,
    /// RPC Port
    /// (If this is not in the message the default is 0)
    #[epee_default(0)]
    pub rpc_port: u16,
    /// RPC Credits Per Hash
    /// (If this is not in the message the default is 0)
    #[epee_default(0)]
    pub rpc_credits_per_hash: u32,
}

/// Core Sync Data, information on the sync state of a peer
#[derive(Debug, Clone, EpeeObject, PartialEq, Eq)]
pub struct CoreSyncData {
    /// Cumulative Difficulty Low
    /// The lower 64 bits of the 128 bit cumulative difficulty
    pub cumulative_difficulty: u64,
    /// Cumulative Difficulty High
    /// The upper 64 bits of the 128 bit cumulative difficulty
    #[epee_default(0)]
    pub cumulative_difficulty_top64: u64,
    /// Current Height of the peer
    pub current_height: u64,
    /// Pruning Seed of the peer
    /// (If this is not in the message the default is 0)
    #[epee_default(0)]
    pub pruning_seed: u32,
    /// Hash of the top block
    pub top_id: [u8; 32],
    /// Version of the top block
    pub top_version: u8,
}

impl CoreSyncData {
    pub fn new(
        cumulative_difficulty_128: u128,
        current_height: u64,
        pruning_seed: u32,
        top_id: [u8; 32],
        top_version: u8,
    ) -> CoreSyncData {
        let cumulative_difficulty = cumulative_difficulty_128 as u64;
        let cumulative_difficulty_top64 = (cumulative_difficulty_128 >> 64) as u64;
        CoreSyncData {
            cumulative_difficulty,
            cumulative_difficulty_top64,
            current_height,
            pruning_seed,
            top_id,
            top_version,
        }
    }
    /// Returns the 128 bit cumulative difficulty of the peers blockchain
    pub fn cumulative_difficulty(&self) -> u128 {
        let mut ret: u128 = self.cumulative_difficulty_top64.into();
        ret <<= 64;
        ret | (Into::<u128>::into(self.cumulative_difficulty))
    }
}

/// PeerListEntryBase, information kept on a peer which will be entered
/// in a peer list/store.
#[derive(Clone, Copy, EpeeObject, Debug, Eq, PartialEq)]
pub struct PeerListEntryBase {
    /// The Peer Address
    pub adr: NetworkAddress,
    /// The Peer ID
    pub id: u64,
    /// The last Time The Peer Was Seen
    #[epee_default(0)]
    pub last_seen: i64,
    /// The Pruning Seed
    #[epee_default(0)]
    pub pruning_seed: u32,
    /// The RPC port
    #[epee_default(0)]
    pub rpc_port: u16,
    /// The RPC credits per hash
    #[epee_default(0)]
    pub rpc_credits_per_hash: u32,
}

impl std::hash::Hash for PeerListEntryBase {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // We only hash the adr so we can look this up in a HashSet.
        self.adr.hash(state)
    }
}

/// A pruned tx with the hash of the missing prunable data
#[derive(Clone, Debug, EpeeObject, PartialEq, Eq)]
pub struct PrunedTxBlobEntry {
    /// The Tx
    pub tx: Vec<u8>,
    /// The Prunable Tx Hash
    pub prunable_hash: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransactionBlobs {
    Pruned(Vec<PrunedTxBlobEntry>),
    Normal(Vec<Vec<u8>>),
}

impl TransactionBlobs {
    pub fn len(&self) -> usize {
        match self {
            TransactionBlobs::Normal(txs) => txs.len(),
            TransactionBlobs::Pruned(txs) => txs.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A Block that can contain transactions
#[derive(Clone, Debug, EpeeObject, PartialEq, Eq)]
pub struct BlockCompleteEntry {
    /// True if tx data is pruned
    #[epee_default(false)]
    pub pruned: bool,
    /// The Block
    pub block: Vec<u8>,
    /// The Block Weight/Size
    #[epee_default(0)]
    pub block_weight: u64,
    /// The blocks txs
    #[epee_default(None)]
    pub txs: Option<TransactionBlobs>,
}

#[cfg(test)]
mod tests {

    use super::CoreSyncData;

    #[test]
    fn core_sync_cumulative_difficulty() {
        let core_sync = CoreSyncData::new(u128::MAX, 80085, 200, [0; 32], 21);
        assert_eq!(core_sync.cumulative_difficulty(), u128::MAX);
        let core_sync = CoreSyncData::new(21, 80085, 200, [0; 32], 21);
        assert_eq!(core_sync.cumulative_difficulty(), 21);
    }
}

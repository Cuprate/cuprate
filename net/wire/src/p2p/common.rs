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

use bitflags::bitflags;

use cuprate_epee_encoding::epee_object;
use cuprate_helper::map::split_u128_into_low_high_bits;
pub use cuprate_types::{BlockCompleteEntry, PrunedTxBlobEntry, TransactionBlobs};

use crate::NetworkAddress;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerSupportFlags(u32);

bitflags! {
    impl PeerSupportFlags: u32 {
        const FLUFFY_BLOCKS = 0b0000_0001;
        const TX_REALY_V2 = 0b0000_0010;
        const _ = !0;
    }
}

impl From<u32> for PeerSupportFlags {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<PeerSupportFlags> for u32 {
    fn from(value: PeerSupportFlags) -> Self {
        value.0
    }
}

impl<'a> From<&'a PeerSupportFlags> for &'a u32 {
    fn from(value: &'a PeerSupportFlags) -> Self {
        &value.0
    }
}

/// Basic Node Data, information on the connected peer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicNodeData {
    /// Port
    pub my_port: u32,
    /// The Network Id
    // We don't use ByteArray here to allow users to keep this data long term.
    pub network_id: [u8; 16],
    /// Peer ID
    pub peer_id: u64,
    /// The Peers Support Flags
    /// (If this is not in the message the default is 0)
    pub support_flags: PeerSupportFlags,
    /// RPC Port
    /// (If this is not in the message the default is 0)
    pub rpc_port: u16,
    /// RPC Credits Per Hash
    /// (If this is not in the message the default is 0)
    pub rpc_credits_per_hash: u32,
}

epee_object! {
    BasicNodeData,
    my_port: u32,
    network_id: [u8; 16],
    peer_id: u64,
    support_flags: PeerSupportFlags as u32 = 0_u32,
    rpc_port: u16 = 0_u16,
    rpc_credits_per_hash: u32 = 0_u32,
}

/// Core Sync Data, information on the sync state of a peer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreSyncData {
    /// Cumulative Difficulty Low
    /// The lower 64 bits of the 128 bit cumulative difficulty
    pub cumulative_difficulty: u64,
    /// Cumulative Difficulty High
    /// The upper 64 bits of the 128 bit cumulative difficulty
    pub cumulative_difficulty_top64: u64,
    /// Current Height of the peer
    pub current_height: u64,
    /// Pruning Seed of the peer
    /// (If this is not in the message the default is 0)
    pub pruning_seed: u32,
    /// Hash of the top block
    // We don't use ByteArray here to allow users to keep this data long term.
    pub top_id: [u8; 32],
    /// Version of the top block
    pub top_version: u8,
}

epee_object! {
    CoreSyncData,
    cumulative_difficulty: u64,
    cumulative_difficulty_top64: u64 = 0_u64,
    current_height: u64,
    pruning_seed: u32 = 0_u32,
    top_id: [u8; 32],
    top_version: u8 = 0_u8,
}

impl CoreSyncData {
    pub const fn new(
        cumulative_difficulty_128: u128,
        current_height: u64,
        pruning_seed: u32,
        top_id: [u8; 32],
        top_version: u8,
    ) -> Self {
        let (cumulative_difficulty, cumulative_difficulty_top64) =
            split_u128_into_low_high_bits(cumulative_difficulty_128);

        Self {
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

/// `PeerListEntryBase`, information kept on a peer which will be entered
/// in a peer list/store.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PeerListEntryBase {
    /// The Peer Address
    pub adr: NetworkAddress,
    /// The Peer ID
    pub id: u64,
    /// The last Time The Peer Was Seen
    pub last_seen: i64,
    /// The Pruning Seed
    pub pruning_seed: u32,
    /// The RPC port
    pub rpc_port: u16,
    /// The RPC credits per hash
    pub rpc_credits_per_hash: u32,
}

epee_object! {
    PeerListEntryBase,
    adr: NetworkAddress,
    id: u64,
    last_seen: i64 = 0_i64,
    pruning_seed: u32 = 0_u32,
    rpc_port: u16 = 0_u16,
    rpc_credits_per_hash: u32 = 0_u32,
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

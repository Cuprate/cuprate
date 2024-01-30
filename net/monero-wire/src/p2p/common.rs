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

use bytes::{Buf, BufMut, Bytes};
use epee_encoding::{epee_object, EpeeValue, InnerMarker};
use fixed_bytes::ByteArray;

use crate::NetworkAddress;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerSupportFlags(u32);

impl From<u32> for PeerSupportFlags {
    fn from(value: u32) -> Self {
        PeerSupportFlags(value)
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

impl PeerSupportFlags {
    //const FLUFFY_BLOCKS: u32 = 0b0000_0001;

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

impl From<u8> for PeerSupportFlags {
    fn from(value: u8) -> Self {
        PeerSupportFlags(value.into())
    }
}

/// Basic Node Data, information on the connected peer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicNodeData {
    /// Port
    pub my_port: u32,
    /// The Network Id
    pub network_id: ByteArray<16>,
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
    network_id: ByteArray<16>,
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
    pub top_id: ByteArray<32>,
    /// Version of the top block
    pub top_version: u8,
}

epee_object! {
    CoreSyncData,
    cumulative_difficulty: u64,
    cumulative_difficulty_top64: u64 = 0_u64,
    current_height: u64,
    pruning_seed: u32 = 0_u32,
    top_id: ByteArray<32>,
    top_version: u8 = 0_u8,
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
            top_id: top_id.into(),
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

impl std::hash::Hash for PeerListEntryBase {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // We only hash the adr so we can look this up in a HashSet.
        self.adr.hash(state)
    }
}

/// A pruned tx with the hash of the missing prunable data
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrunedTxBlobEntry {
    /// The Tx
    pub tx: Bytes,
    /// The Prunable Tx Hash
    pub prunable_hash: ByteArray<32>,
}

epee_object!(
    PrunedTxBlobEntry,
    tx: Bytes,
    prunable_hash: ByteArray<32>,
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransactionBlobs {
    Pruned(Vec<PrunedTxBlobEntry>),
    Normal(Vec<Bytes>),
    None,
}

impl TransactionBlobs {
    pub fn take_pruned(self) -> Option<Vec<PrunedTxBlobEntry>> {
        match self {
            TransactionBlobs::Normal(_) => None,
            TransactionBlobs::Pruned(txs) => Some(txs),
            TransactionBlobs::None => Some(vec![]),
        }
    }

    pub fn take_normal(self) -> Option<Vec<Bytes>> {
        match self {
            TransactionBlobs::Normal(txs) => Some(txs),
            TransactionBlobs::Pruned(_) => None,
            TransactionBlobs::None => Some(vec![]),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            TransactionBlobs::Normal(txs) => txs.len(),
            TransactionBlobs::Pruned(txs) => txs.len(),
            TransactionBlobs::None => 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A Block that can contain transactions
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockCompleteEntry {
    /// True if tx data is pruned
    pub pruned: bool,
    /// The Block
    pub block: Bytes,
    /// The Block Weight/Size
    pub block_weight: u64,
    /// The blocks txs
    pub txs: TransactionBlobs,
}

epee_object!(
    BlockCompleteEntry,
    pruned: bool = false,
    block: Bytes,
    block_weight: u64 = 0_u64,
    txs: TransactionBlobs = TransactionBlobs::None => tx_blob_read, tx_blob_write, should_write_tx_blobs,
);

fn tx_blob_read<B: Buf>(b: &mut B) -> epee_encoding::Result<TransactionBlobs> {
    let marker = epee_encoding::read_marker(b)?;
    match marker.inner_marker {
        InnerMarker::Object => Ok(TransactionBlobs::Pruned(Vec::read(b, &marker)?)),
        InnerMarker::String => Ok(TransactionBlobs::Normal(Vec::read(b, &marker)?)),
        _ => Err(epee_encoding::Error::Value("Invalid marker for tx blobs")),
    }
}

fn tx_blob_write<B: BufMut>(
    val: TransactionBlobs,
    field_name: &str,
    w: &mut B,
) -> epee_encoding::Result<()> {
    if should_write_tx_blobs(&val) {
        match val {
            TransactionBlobs::Normal(bytes) => epee_encoding::write_field(bytes, field_name, w)?,
            TransactionBlobs::Pruned(obj) => epee_encoding::write_field(obj, field_name, w)?,
            TransactionBlobs::None => (),
        }
    }
    Ok(())
}

fn should_write_tx_blobs(val: &TransactionBlobs) -> bool {
    !val.is_empty()
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

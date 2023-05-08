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

use epee_serde::Value;
use serde::de;
use serde::ser::{SerializeSeq, SerializeStruct};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::TryFromInto;

use crate::utils;
use crate::NetworkAddress;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct PeerSupportFlags(u32); // had to name it this to avoid conflict

impl PeerSupportFlags {
    const FLUFFY_BLOCKS: u32 = 0b0000_0001;
    /// checks if `self` has all the flags that `other` has
    pub fn contains(&self, other: &PeerSupportFlags) -> bool {
        self.0 & other.0 == other.0
    }
    pub fn supports_fluffy_blocks(&self) -> bool {
        self.0 & Self::FLUFFY_BLOCKS == Self::FLUFFY_BLOCKS
    }
    pub const fn get_support_flag_fluffy_blocks() -> Self {
        PeerSupportFlags(Self::FLUFFY_BLOCKS)
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

impl From<u8> for PeerSupportFlags {
    fn from(value: u8) -> Self {
        PeerSupportFlags(value as u32)
    }
}

impl From<u32> for PeerSupportFlags {
    fn from(value: u32) -> Self {
        PeerSupportFlags(value)
    }
}

/// A PeerID, different from a `NetworkAddress`
#[derive(Debug, Clone, Default, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct PeerID(pub u64);

/// Basic Node Data, information on the connected peer
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct BasicNodeData {
    /// Port
    pub my_port: u32,
    /// The Network Id
    pub network_id: [u8; 16],
    /// Peer ID
    pub peer_id: PeerID,
    /// The Peers Support Flags
    /// (If this is not in the message the default is 0)
    #[serde(default = "utils::zero_val")]
    pub support_flags: PeerSupportFlags,
    /// RPC Port
    /// (If this is not in the message the default is 0)
    #[serde(default = "utils::zero_val")]
    pub rpc_port: u16,
    /// RPC Credits Per Hash
    /// (If this is not in the message the default is 0)
    #[serde(default = "utils::zero_val")]
    pub rpc_credits_per_hash: u32,
}

/// Core Sync Data, information on the sync state of a peer
#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CoreSyncData {
    /// Cumulative Difficulty Low
    /// The lower 64 bits of the 128 bit cumulative difficulty
    pub cumulative_difficulty: u64,
    /// Cumulative Difficulty High
    /// The upper 64 bits of the 128 bit cumulative difficulty
    #[serde(default = "utils::zero_val")]
    pub cumulative_difficulty_top64: u64,
    /// Current Height of the peer
    pub current_height: u64,
    /// Pruning Seed of the peer
    /// (If this is not in the message the default is 0)
    #[serde(default = "utils::zero_val")]
    pub pruning_seed: u32,
    /// Hash of the top block
    #[serde_as(as = "TryFromInto<[u8; 32]>")]
    pub top_id: [u8; 32],
    /// Version of the top block
    #[serde(default = "utils::zero_val")]
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
        let mut ret: u128 = self.cumulative_difficulty_top64 as u128;
        ret <<= 64;
        ret | self.cumulative_difficulty as u128
    }
}

/// PeerListEntryBase, information kept on a peer which will be entered
/// in a peer list/store.
#[derive(Clone, Copy, Default, Deserialize, Serialize, Debug, Eq, PartialEq)]
pub struct PeerListEntryBase {
    /// The Peer Address
    pub adr: NetworkAddress,
    /// The Peer ID
    pub id: PeerID,
    /// The last Time The Peer Was Seen
    #[serde(default = "utils::zero_val")]
    pub last_seen: i64,
    /// The Pruning Seed
    #[serde(default = "utils::zero_val")]
    pub pruning_seed: u32,
    /// The RPC port
    #[serde(default = "utils::zero_val")]
    pub rpc_port: u16,
    /// The RPC credits per hash
    #[serde(default = "utils::zero_val")]
    pub rpc_credits_per_hash: u32,
}

/// A pruned tx with the hash of the missing prunable data
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrunedTxBlobEntry {
    /// The Tx
    pub tx: Vec<u8>,
    /// The Prunable Tx Hash
    pub prunable_hash: [u8; 32],
}

impl PrunedTxBlobEntry {
    fn from_epee_value<E: de::Error>(mut value: Value) -> Result<Self, E> {
        let tx = utils::get_internal_val_from_map(&mut value, "blob", Value::get_bytes, "Vec<u8>")?;

        let prunable_hash = utils::get_internal_val_from_map(
            &mut value,
            "prunable_hash",
            Value::get_bytes,
            "Vec<u8>",
        )?;
        let prunable_hash_len = prunable_hash.len();

        Ok(PrunedTxBlobEntry {
            tx,
            prunable_hash: prunable_hash
                .try_into()
                .map_err(|_| E::invalid_length(prunable_hash_len, &"a 16-byte array"))?,
        })
    }
}

impl Serialize for PrunedTxBlobEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("", 2)?;
        state.serialize_field("blob", &self.tx)?;
        state.serialize_field("prunable_hash", &self.prunable_hash)?;
        state.end()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransactionBlobs {
    Pruned(Vec<PrunedTxBlobEntry>),
    Normal(Vec<Vec<u8>>),
}

impl TransactionBlobs {
    pub fn new_unpruned(txs: Vec<Vec<u8>>) -> Self {
        TransactionBlobs::Normal(txs)
    }

    pub fn len(&self) -> usize {
        match self {
            TransactionBlobs::Normal(txs) => txs.len(),
            TransactionBlobs::Pruned(txs) => txs.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn from_epee_value<E: de::Error>(value: Value, pruned: bool) -> Result<Self, E> {
        let txs = utils::get_internal_val(value, Value::get_seq, "A sequence")?;
        if pruned {
            let mut decoded_txs = Vec::with_capacity(txs.len());
            for tx in txs {
                decoded_txs.push(PrunedTxBlobEntry::from_epee_value(tx)?);
            }
            Ok(TransactionBlobs::Pruned(decoded_txs))
        } else {
            let mut decoded_txs = Vec::with_capacity(txs.len());
            for tx in txs {
                decoded_txs.push(utils::get_internal_val(tx, Value::get_bytes, "Vec<u8>")?);
            }
            Ok(TransactionBlobs::Normal(decoded_txs))
        }
    }
}

impl Serialize for TransactionBlobs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            TransactionBlobs::Pruned(txs) => {
                let mut seq = serializer.serialize_seq(Some(txs.len()))?;

                for tx in txs {
                    seq.serialize_element(tx)?;
                }

                seq.end()
            }
            TransactionBlobs::Normal(txs) => {
                let mut seq = serializer.serialize_seq(Some(txs.len()))?;

                for tx in txs {
                    seq.serialize_element(tx)?;
                }

                seq.end()
            }
        }
    }
}

/// A Block that can contain transactions
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockCompleteEntry {
    /// True if tx data is pruned
    pub pruned: bool,
    /// The Block
    pub block: Vec<u8>,
    /// The Block Weight/Size
    pub block_weight: u64,
    /// The blocks txs
    pub txs: TransactionBlobs,
}

impl<'de> Deserialize<'de> for BlockCompleteEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut value = Value::deserialize(deserializer)?;
        let mut pruned = false;
        if let Some(val) = value.get_and_remove("pruned") {
            pruned = utils::get_internal_val(val, Value::get_bool, "bool")?;
        }

        let block =
            utils::get_internal_val_from_map(&mut value, "block", Value::get_bytes, "Vec<u8>")?;

        let mut block_weight = 0;

        let txs_value = value.get_and_remove("txs");

        let mut txs = TransactionBlobs::Normal(vec![]);

        if let Some(txs_value) = txs_value {
            txs = TransactionBlobs::from_epee_value(txs_value, true)?;
        }

        if pruned {
            block_weight = utils::get_internal_val_from_map(
                &mut value,
                "block_weight",
                Value::get_u64,
                "u64",
            )?;
        }

        Ok(BlockCompleteEntry {
            pruned,
            block,
            block_weight,
            txs,
        })
    }
}

impl Serialize for BlockCompleteEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut len = 1;
        if !self.txs.is_empty() {
            len += 1;
        }
        if self.pruned {
            // one field to store the value of `pruned`
            // another to sore the block weight
            len += 2;
        }

        let mut state = serializer.serialize_struct("", len)?;

        state.serialize_field("block", &self.block)?;

        if self.pruned {
            state.serialize_field("pruned", &true)?;
            state.serialize_field("block_weight", &self.block_weight)?;
        }

        if !self.txs.is_empty() {
            state.serialize_field("txs", &self.txs)?;
        }

        state.end()
    }
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

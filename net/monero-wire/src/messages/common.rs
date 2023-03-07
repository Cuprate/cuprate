//! Common types that are used across multiple messages.
//

use epee_serde::Value;
use monero::{Block, Hash, Transaction};
use serde::de;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::TryFromInto;

use super::zero_val;
use crate::NetworkAddress;

/// A PeerID, different from a `NetworkAddress`
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
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
    #[serde(default = "zero_val")]
    pub support_flags: u32,
    /// RPC Port
    /// (If this is not in the message the default is 0)
    #[serde(default = "zero_val")]
    pub rpc_port: u16,
    /// RPC Credits Per Hash
    /// (If this is not in the message the default is 0)
    #[serde(default = "zero_val")]
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
    #[serde(default = "zero_val")]
    pub cumulative_difficulty_top64: u64,
    /// Current Height of the peer
    pub current_height: u64,
    /// Pruning Seed of the peer
    /// (If this is not in the message the default is 0)
    #[serde(default = "zero_val")]
    pub pruning_seed: u32,
    /// Hash of the top block
    #[serde_as(as = "TryFromInto<[u8; 32]>")]
    pub top_id: Hash,
    /// Version of the top block
    #[serde(default = "zero_val")]
    pub top_version: u8,
}

impl CoreSyncData {
    /// Returns the 128 bit cumulative difficulty of the peers blockchain
    pub fn cumulative_difficulty(&self) -> u128 {
        let mut ret: u128 = self.cumulative_difficulty_top64 as u128;
        ret <<= 64;
        ret | self.cumulative_difficulty as u128
    }
}

/// PeerListEntryBase, information kept on a peer which will be entered
/// in a peer list/store.
#[derive(Clone, Copy, Deserialize, Serialize, Debug, Eq, PartialEq)]
pub struct PeerListEntryBase {
    /// The Peer Address
    pub adr: NetworkAddress,
    /// The Peer ID
    pub id: PeerID,
    /// The last Time The Peer Was Seen
    #[serde(default = "zero_val")]
    pub last_seen: i64,
    /// The Pruning Seed
    #[serde(default = "zero_val")]
    pub pruning_seed: u32,
    /// The RPC port
    #[serde(default = "zero_val")]
    pub rpc_port: u16,
    /// The RPC credits per hash
    #[serde(default = "zero_val")]
    pub rpc_credits_per_hash: u32,
}

/// A pruned tx with the hash of the missing prunable data
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TxBlobEntry {
    /// The Tx
    pub tx: Transaction, // ########### use pruned transaction when PR is merged ##############
    /// The Prunable Tx Hash
    pub prunable_hash: Hash,
}

impl TxBlobEntry {
    fn from_epee_value<E: de::Error>(value: &Value) -> Result<Self, E> {
        let tx_blob = get_val_from_map!(value, "blob", get_bytes, "Vec<u8>");

        let tx = monero_decode_into_serde_err!(Transaction, tx_blob);

        let prunable_hash_blob = get_val_from_map!(value, "prunable_hash", get_bytes, "Vec<u8>");

        let prunable_hash = Hash::from_slice(prunable_hash_blob);

        Ok(Self { tx, prunable_hash })
    }
}

impl Serialize for TxBlobEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("", 2)?;
        let tx_blob = monero::consensus::serialize(&self.tx);
        state.serialize_field("blob", &tx_blob)?;
        let prunable_hash = self.prunable_hash.as_bytes();
        state.serialize_field("prunable_hash", prunable_hash)?;
        state.end()
    }
}

/// A Block that can contain transactions
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockCompleteEntry {
    /// True if tx data is pruned
    pub pruned: bool,
    /// The Block
    pub block: Block,
    /// The Block Weight/Size
    pub block_weight: u64,
    /// If the Block is pruned the txs will be here
    pub txs_pruned: Vec<TxBlobEntry>,
    /// If the Block is not pruned the txs will be here
    pub txs: Vec<Transaction>,
}

impl<'de> Deserialize<'de> for BlockCompleteEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let mut pruned = false;
        if let Some(val) = value.get("pruned") {
            pruned = *get_internal_val!(val, get_bool, "bool");
        }

        let block_bytes = get_val_from_map!(value, "block", get_bytes, "Vec<u8>");

        let block = monero_decode_into_serde_err!(Block, block_bytes);

        let mut block_weight = 0;

        let mut txs_pruned = vec![];
        let mut txs = vec![];

        if pruned {
            block_weight = *get_val_from_map!(value, "block_weight", get_u64, "u64");

            if let Some(v) = value.get("txs") {
                let v = get_internal_val!(v, get_seq, "a sequence");

                txs_pruned.reserve(v.len());
                for val in v {
                    txs_pruned.push(TxBlobEntry::from_epee_value(val)?);
                }
            }
        } else if let Some(v) = value.get("txs") {
            let v = get_internal_val!(v, get_seq, "a sequence");

            txs.reserve(v.len());
            for val in v {
                let tx_buf = get_internal_val!(val, get_bytes, "Vec<u8>");
                txs.push(monero_decode_into_serde_err!(Transaction, tx_buf));
            }
        }
        Ok(BlockCompleteEntry {
            pruned,
            block,
            block_weight,
            txs_pruned,
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
        if !self.txs.is_empty() || !self.txs_pruned.is_empty() {
            len += 1;
        }
        if self.pruned {
            // one field to store the value of `pruned`
            // another to sore the block weight
            len += 2;
        }

        let mut state = serializer.serialize_struct("", len)?;

        let block = monero::consensus::serialize(&self.block);
        state.serialize_field("block", &block)?;

        if self.pruned {
            state.serialize_field("pruned", &true)?;
            state.serialize_field("block_weight", &self.block_weight)?;

            if !self.txs_pruned.is_empty() {
                state.serialize_field("txs", &self.txs_pruned)?;
            }
        } else if !self.txs.is_empty() {
            let mut tx_blobs = vec![];
            for tx in self.txs.iter() {
                tx_blobs.push(monero::consensus::serialize(tx));
            }
            state.serialize_field("txs", &tx_blobs)?;
        }

        state.end()
    }
}

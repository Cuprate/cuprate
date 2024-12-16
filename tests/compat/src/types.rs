use hex::serde::deserialize;
use monero_serai::{block::Block, transaction::Transaction};
use serde::Deserialize;

/// Data of a single block from RPC.
#[derive(Debug)]
pub struct RpcBlockData {
    /// Subset of JSON-RPC `get_block` data.
    pub get_block_response: GetBlockResponse,

    /// The block itself.
    pub block: Block,

    /// The correct seed height needed for this block for `RandomX`.
    pub seed_height: u64,
    /// The correct seed hash needed for this block for `RandomX`.
    pub seed_hash: [u8; 32],

    /// All transactions in the block.
    /// This vec is:
    /// - the original transaction blobs
    pub txs: Vec<RpcTxData>,
}

/// Data of a transaction.
#[derive(Debug)]
pub struct RpcTxData {
    /// The transactions itself.
    pub tx: Transaction,
    /// The transactions blob.
    pub tx_blob: Vec<u8>,
    /// The transaction's hash.
    pub tx_hash: [u8; 32],
}

/// Subset of JSON-RPC `get_block` response.
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse {
    pub result: GetBlockResponse,
}

/// Subset of JSON-RPC `get_block` data.
#[derive(Debug, Clone, Deserialize)]
pub struct GetBlockResponse {
    #[serde(deserialize_with = "deserialize")]
    pub blob: Vec<u8>,
    pub block_header: BlockHeader,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub(crate) struct BlockHeader {
    #[serde(deserialize_with = "deserialize")]
    pub hash: [u8; 32],
    #[serde(deserialize_with = "deserialize")]
    pub pow_hash: [u8; 32],
    #[serde(deserialize_with = "deserialize")]
    pub miner_tx_hash: [u8; 32],
    #[serde(deserialize_with = "deserialize")]
    pub prev_hash: [u8; 32],

    pub block_weight: usize,
    pub height: u64,
    pub major_version: u8,
    pub minor_version: u8,
    pub nonce: u32,
    pub num_txes: usize,
    pub reward: u64,
    pub timestamp: u64,
}

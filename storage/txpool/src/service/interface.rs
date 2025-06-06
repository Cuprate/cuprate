//! Tx-pool [`service`](super) interface.
//!
//! This module contains `cuprate_txpool`'s [`tower::Service`] request and response enums.

use std::{
    collections::{HashMap, HashSet},
    num::NonZero,
};

use cuprate_types::{
    rpc::{PoolInfo, SpentKeyImageInfo, TxInfo, TxpoolStats},
    TransactionVerificationData, TxInPool,
};

use crate::{
    tx::TxEntry,
    types::{KeyImage, TransactionBlobHash, TransactionHash},
};

//---------------------------------------------------------------------------------------------------- TxpoolReadRequest
/// The transaction pool [`tower::Service`] read request type.
///
/// ### `include_sensitive_txs`
/// This field exists in many requests.
/// If this is [`true`], the request will include private (local) transactions in the response.
#[derive(Clone)]
pub enum TxpoolReadRequest {
    /// Get the blob (raw bytes) of a transaction with the given hash.
    TxBlob(TransactionHash),

    /// Get the [`TransactionVerificationData`] of a transaction in the tx pool.
    TxVerificationData(TransactionHash),

    /// Filter (remove) all **known** transactions from the set.
    ///
    /// The hash is **not** the transaction hash, it is the hash of the serialized tx-blob.
    FilterKnownTxBlobHashes(HashSet<TransactionBlobHash>),

    /// Get some transactions for an incoming block.
    TxsForBlock(Vec<TransactionHash>),

    /// Get information on all transactions in the pool.
    Backlog,

    /// Get the number of transactions in the pool.
    Size { include_sensitive_txs: bool },

    /// Get general information on the txpool.
    PoolInfo {
        include_sensitive_txs: bool,
        /// The maximum amount of transactions to retrieve.
        max_tx_count: usize,
        /// Fetch transactions that start from this time.
        ///
        /// [`None`] means all transactions.
        start_time: Option<NonZero<usize>>,
    },

    /// Get transactions by their hashes.
    TxsByHash {
        tx_hashes: Vec<[u8; 32]>,
        include_sensitive_txs: bool,
    },

    /// Check if any individual key images of a set exist in the txpool.
    KeyImagesSpent {
        key_images: HashSet<[u8; 32]>,
        include_sensitive_txs: bool,
    },

    /// Same as [`TxpoolReadRequest::KeyImagesSpent`] but with a [`Vec`].
    ///
    /// The response will be in the same order as the request.
    KeyImagesSpentVec {
        key_images: Vec<[u8; 32]>,
        include_sensitive_txs: bool,
    },

    /// Get txpool info.
    Pool { include_sensitive_txs: bool },

    /// Get txpool stats.
    PoolStats { include_sensitive_txs: bool },

    /// Get the hashes of all transaction in the pool.
    AllHashes { include_sensitive_txs: bool },
}

//---------------------------------------------------------------------------------------------------- TxpoolReadResponse
/// The transaction pool [`tower::Service`] read response type.
#[expect(clippy::large_enum_variant)]
pub enum TxpoolReadResponse {
    /// The response for [`TxpoolReadRequest::TxBlob`].
    TxBlob { tx_blob: Vec<u8>, state_stem: bool },

    /// The response for [`TxpoolReadRequest::TxVerificationData`].
    TxVerificationData(TransactionVerificationData),

    /// The response for [`TxpoolReadRequest::FilterKnownTxBlobHashes`].
    FilterKnownTxBlobHashes {
        /// The blob hashes that are unknown.
        unknown_blob_hashes: HashSet<TransactionBlobHash>,
        /// The tx hashes of the blob hashes that were known but were in the stem pool.
        stem_pool_hashes: Vec<TransactionHash>,
    },

    /// The response for [`TxpoolReadRequest::TxsForBlock`].
    TxsForBlock {
        /// The txs we had in the txpool.
        txs: HashMap<[u8; 32], TransactionVerificationData>,
        /// The indexes of the missing txs.
        missing: Vec<usize>,
    },

    /// Response to [`TxpoolReadRequest::Backlog`].
    ///
    /// The inner [`Vec`] contains information on all
    /// the transactions currently in the pool.
    Backlog(Vec<TxEntry>),

    /// Response to [`TxpoolReadRequest::Size`].
    ///
    /// The inner value is the amount of
    /// transactions currently in the pool.
    Size(usize),

    /// Response to [`TxpoolReadRequest::PoolInfo`].
    PoolInfo(PoolInfo),

    /// Response to [`TxpoolReadRequest::TxsByHash`].
    TxsByHash(Vec<TxInPool>),

    /// Response to [`TxpoolReadRequest::KeyImagesSpent`].
    KeyImagesSpent(bool),

    /// Response to [`TxpoolReadRequest::KeyImagesSpentVec`].
    ///
    /// Inner value is a `Vec` the same length as the input.
    ///
    /// The index of each entry corresponds with the request.
    /// `true` means that the key image was spent.
    KeyImagesSpentVec(Vec<bool>),

    /// Response to [`TxpoolReadRequest::Pool`].
    Pool {
        txs: Vec<TxInfo>,
        spent_key_images: Vec<SpentKeyImageInfo>,
    },

    /// Response to [`TxpoolReadRequest::PoolStats`].
    PoolStats(TxpoolStats),

    /// Response to [`TxpoolReadRequest::AllHashes`].
    AllHashes(Vec<[u8; 32]>),
}

//---------------------------------------------------------------------------------------------------- TxpoolWriteRequest
/// The transaction pool [`tower::Service`] write request type.
#[derive(Clone)]
pub enum TxpoolWriteRequest {
    /// Add a transaction to the pool.
    ///
    /// Returns [`TxpoolWriteResponse::AddTransaction`].
    AddTransaction {
        /// The tx to add.
        tx: Box<TransactionVerificationData>,
        /// A [`bool`] denoting the routing state of this tx.
        ///
        /// [`true`] if this tx is in the stem state.
        state_stem: bool,
    },

    /// Remove a transaction with the given hash from the pool.
    RemoveTransaction(TransactionHash),

    /// Promote a transaction from the stem pool to the fluff pool.
    /// If the tx is already in the fluff pool this does nothing.
    Promote(TransactionHash),

    /// Tell the tx-pool about a new block.
    NewBlock {
        /// The spent key images in the new block.
        spent_key_images: Vec<KeyImage>,
    },
}

//---------------------------------------------------------------------------------------------------- TxpoolWriteResponse
/// The transaction pool [`tower::Service`] write response type.
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum TxpoolWriteResponse {
    /// Response to:
    /// - [`TxpoolWriteRequest::RemoveTransaction`]
    /// - [`TxpoolWriteRequest::Promote`]
    Ok,

    /// Response to [`TxpoolWriteRequest::NewBlock`].
    ///
    /// The inner values are the transactions removed from the pool.
    NewBlock(Vec<TransactionHash>),

    /// Response to [`TxpoolWriteRequest::AddTransaction`].
    ///
    /// If the inner value is [`Some`] the tx was not added to the pool as it double spends a tx with the given hash.
    AddTransaction(Option<TransactionHash>),
}

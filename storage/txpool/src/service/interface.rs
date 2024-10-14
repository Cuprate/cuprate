//! Tx-pool [`service`](super) interface.
//!
//! This module contains `cuprate_txpool`'s [`tower::Service`] request and response enums.
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use cuprate_types::TransactionVerificationData;

use crate::types::{KeyImage, TransactionBlobHash, TransactionHash};

//---------------------------------------------------------------------------------------------------- TxpoolReadRequest
/// The transaction pool [`tower::Service`] read request type.
#[derive(Clone)]
pub enum TxpoolReadRequest {
    /// A request for the blob (raw bytes) of a transaction with the given hash.
    TxBlob(TransactionHash),
    /// A request for the [`TransactionVerificationData`] of a transaction in the tx pool.
    TxVerificationData(TransactionHash),
    /// A request to filter (remove) all **known** transactions from the set.
    ///
    /// The hash is **not** the transaction hash, it is the hash of the serialized tx-blob.
    FilterKnownTxBlobHashes(HashSet<TransactionBlobHash>),
    /// A request to pull some transactions for an incoming block.
    TxsForBlock(Vec<TransactionHash>),
}

//---------------------------------------------------------------------------------------------------- TxpoolReadResponse
/// The transaction pool [`tower::Service`] read response type.
#[expect(clippy::large_enum_variant)]
pub enum TxpoolReadResponse {
    /// A response containing the raw bytes of a transaction.
    TxBlob { tx_blob: Vec<u8>, state_stem: bool },
    /// A response of [`TransactionVerificationData`].
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
        tx: Arc<TransactionVerificationData>,
        /// A [`bool`] denoting the routing state of this tx.
        ///
        /// [`true`] if this tx is in the stem state.
        state_stem: bool,
    },
    /// Remove a transaction with the given hash from the pool.
    ///
    /// Returns [`TxpoolWriteResponse::Ok`].
    RemoveTransaction(TransactionHash),
    /// Promote a transaction from the stem pool to the fluff pool.
    /// If the tx is already in the fluff pool this does nothing.
    ///
    /// Returns [`TxpoolWriteResponse::Ok`].
    Promote(TransactionHash),
    /// Tell the tx-pool about a new block.
    NewBlock {
        /// The new blockchain height.
        blockchain_height: usize,
        /// The spent key images in the new block.
        spent_key_images: Vec<KeyImage>,
    },
}

//---------------------------------------------------------------------------------------------------- TxpoolWriteResponse
/// The transaction pool [`tower::Service`] write response type.
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum TxpoolWriteResponse {
    /// A [`TxpoolWriteRequest::AddTransaction`] response.
    ///
    /// If the inner value is [`Some`] the tx was not added to the pool as it double spends a tx with the given hash.
    AddTransaction(Option<TransactionHash>),
    Ok,
}

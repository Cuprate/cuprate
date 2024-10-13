//! Tx-pool [`service`](super) interface.
//!
//! This module contains `cuprate_txpool`'s [`tower::Service`] request and response enums.
use std::{collections::HashSet, sync::Arc};

use cuprate_types::TransactionVerificationData;

use crate::types::{TransactionBlobHash, TransactionHash};

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
    FilterKnownTxBlobHashes(HashSet<TransactionBlobHash>),
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

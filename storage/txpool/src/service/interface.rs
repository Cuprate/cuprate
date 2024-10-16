//! Tx-pool [`service`](super) interface.
//!
//! This module contains `cuprate_txpool`'s [`tower::Service`] request and response enums.
use std::sync::Arc;

use cuprate_types::TransactionVerificationData;

use crate::{tx::TxEntry, types::TransactionHash};

//---------------------------------------------------------------------------------------------------- TxpoolReadRequest
/// The transaction pool [`tower::Service`] read request type.
pub enum TxpoolReadRequest {
    /// A request for the blob (raw bytes) of a transaction with the given hash.
    TxBlob(TransactionHash),

    /// A request for the [`TransactionVerificationData`] of a transaction in the tx pool.
    TxVerificationData(TransactionHash),

    /// Get information on all transactions in the pool.
    Backlog,

    /// Get the number of transactions in the pool.
    Size {
        /// TODO
        include_sensitive_txs: bool,
    },
}

//---------------------------------------------------------------------------------------------------- TxpoolReadResponse
/// The transaction pool [`tower::Service`] read response type.
#[expect(clippy::large_enum_variant)]
pub enum TxpoolReadResponse {
    /// Response to [`TxpoolReadRequest::TxBlob`].
    ///
    /// The inner value is the raw bytes of a transaction.
    // TODO: use bytes::Bytes.
    TxBlob(Vec<u8>),

    /// Response to [`TxpoolReadRequest::TxVerificationData`].
    TxVerificationData(TransactionVerificationData),

    /// Response to [`TxpoolReadRequest::Backlog`].
    ///
    /// The inner `Vec` contains information on all
    /// the transactions currently in the pool.
    Backlog(Vec<TxEntry>),

    /// Response to [`TxpoolReadRequest::Size`].
    ///
    /// The inner value is the amount of
    /// transactions currently in the pool.
    Size(usize),
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
}

//---------------------------------------------------------------------------------------------------- TxpoolWriteResponse
/// The transaction pool [`tower::Service`] write response type.
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum TxpoolWriteResponse {
    /// Response to:
    /// - [`TxpoolWriteRequest::RemoveTransaction`]
    Ok,

    /// Response to [`TxpoolWriteRequest::AddTransaction`].
    ///
    /// If the inner value is [`Some`] the tx was not added to the pool as it double spends a tx with the given hash.
    AddTransaction(Option<TransactionHash>),
}

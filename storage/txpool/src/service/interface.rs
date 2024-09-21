//! Tx-pool [`service`](super) interface.
//!
//! This module contains `cuprate_txpool`'s [`tower::Service`] request and response enums.
use std::sync::Arc;

use cuprate_types::TransactionVerificationData;

use crate::types::TransactionHash;

//---------------------------------------------------------------------------------------------------- TxpoolReadRequest
/// The transaction pool [`tower::Service`] read request type.
pub enum TxpoolReadRequest {
    /// A request for the blob (raw bytes) of a transaction with the given hash.
    TxBlob(TransactionHash),
    /// A request for the [`TransactionVerificationData`] of a transaction in the tx pool.
    TxVerificationData(TransactionHash),
}

//---------------------------------------------------------------------------------------------------- TxpoolReadResponse
/// The transaction pool [`tower::Service`] read response type.
<<<<<<< HEAD
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
||||||| 0162553
#[allow(clippy::large_enum_variant)]
=======
#[expect(clippy::large_enum_variant)]
>>>>>>> main
pub enum TxpoolReadResponse {
    /// A response containing the raw bytes of a transaction.
    // TODO: use bytes::Bytes.
    TxBlob(Vec<u8>),
    /// A response of [`TransactionVerificationData`].
    TxVerificationData(TransactionVerificationData),
}

//---------------------------------------------------------------------------------------------------- TxpoolWriteRequest
/// The transaction pool [`tower::Service`] write request type.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum TxpoolWriteResponse {
    /// A [`TxpoolWriteRequest::AddTransaction`] response.
    ///
    /// If the inner value is [`Some`] the tx was not added to the pool as it double spends a tx with the given hash.
    AddTransaction(Option<TransactionHash>),
    Ok,
}

use crate::types::TransactionHash;
use cuprate_types::TransactionVerificationData;
use std::sync::Arc;

//---------------------------------------------------------------------------------------------------- TxpoolReadRequest
/// The transaction pool [`tower::Service`] request type.
pub enum TxpoolReadRequest {
    /// A request for the blob (raw bytes) of a transaction with the given hash.
    TxBlob(TransactionHash),
    /// A request for the [`TransactionVerificationData`] of a transaction in the tx pool.
    TxVerificationData(TransactionHash),
}

//---------------------------------------------------------------------------------------------------- TxpoolReadResponse
pub enum TxpoolReadResponse {
    /// A response containing the raw bytes of a transaction.
    // TODO: use bytes::Bytes.
    TxBlob(Vec<u8>),
    /// A response of [`TransactionVerificationData`].
    TxVerificationData(TransactionVerificationData),
}

//---------------------------------------------------------------------------------------------------- TxpoolWriteRequest
pub enum TxpoolWriteRequest {
    AddTransaction {
        tx: Arc<TransactionVerificationData>,
        state_fluff: bool,
    },
    RemoveTransaction(TransactionHash),
    PromoteTransactionToFluffPool(TransactionHash),
}

//---------------------------------------------------------------------------------------------------- TxpoolWriteResponse
pub enum TxpoolWriteResponse {}

use crate::types::TransactionHash;
use cuprate_dandelion_tower::State;
use cuprate_types::TransactionVerificationData;
use std::sync::Arc;

//---------------------------------------------------------------------------------------------------- TxpoolReadRequest
/// The transaction pool [`tower::Service`] request type.
pub enum TxpoolReadRequest {
    /// A request for the blob (raw bytes) of a transaction with the given hash.
    TxBlob(TransactionHash),
    /// A request for the [`TransactionVerificationData`] of a transaction in the tx pool.
    TxVerificationData(TransactionHash),
    /// Returns if we have a transaction in the pool.
    TxInPool(TransactionHash),
}

//---------------------------------------------------------------------------------------------------- TxpoolReadResponse
pub enum TxpoolReadResponse {
    /// A response containing the raw bytes of a transaction.
    // TODO: use bytes::Bytes.
    TxBlob(Vec<u8>),
    /// A response of [`TransactionVerificationData`].
    TxVerificationData(TransactionVerificationData),
    TxInPool(Option<State>),
}

//---------------------------------------------------------------------------------------------------- TxpoolWriteRequest
pub enum TxpoolWriteRequest {
    AddTransaction(NewTransaction),
    RemoveTransaction(TransactionHash),
    PromoteTransactionToFluffPool(TransactionHash),
}

//---------------------------------------------------------------------------------------------------- TxpoolWriteResponse
pub enum TxpoolWriteResponse {}

pub struct NewTransaction {
    tx: Arc<TransactionVerificationData>,
    dpp_state: State,
}

use crate::types::TransactionHash;
use cuprate_types::TransactionVerificationData;
use std::sync::Arc;

//---------------------------------------------------------------------------------------------------- TxpoolReadRequest
pub enum TxpoolReadRequest {
    GetTransaction(TransactionHash),
    TransactionInPool(TransactionHash),
}

//---------------------------------------------------------------------------------------------------- TxpoolReadResponse
pub enum TxpoolReadResponse {
    GetTransaction {},
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

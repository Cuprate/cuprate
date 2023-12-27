use std::{
    collections::{HashMap, HashSet},
    future::Future,
    sync::Arc,
};

use monero_consensus::{transactions::OutputOnChain, ConsensusError, HardFork};

//mod batch_verifier;
mod batch_verifier;
pub mod block;
pub mod context;
mod helper;
#[cfg(feature = "binaries")]
pub mod rpc;
#[cfg(test)]
mod test_utils;
pub mod transactions;

pub use block::{
    PrePreparedBlock, VerifiedBlockInformation, VerifyBlockRequest, VerifyBlockResponse,
};
pub use context::{
    initialize_blockchain_context, BlockChainContext, BlockChainContextRequest,
    BlockChainContextResponse, ContextConfig,
};
pub use transactions::{VerifyTxRequest, VerifyTxResponse};

#[derive(Debug, thiserror::Error)]
pub enum ExtendedConsensusError {
    #[error("{0}")]
    ConErr(#[from] monero_consensus::ConsensusError),
    #[error("Database error: {0}")]
    DBErr(#[from] tower::BoxError),
    #[error("Needed transaction is not in pool")]
    TxPErr(#[from] TxNotInPool),
}

// TODO: instead of (ab)using generic returns return the acc type
pub async fn initialize_verifier<D, TxP, Ctx>(
    database: D,
    tx_pool: TxP,
    ctx_svc: Ctx,
) -> Result<
    (
        impl tower::Service<
                VerifyBlockRequest,
                Response = VerifyBlockResponse,
                Error = ExtendedConsensusError,
                Future = impl Future<Output = Result<VerifyBlockResponse, ExtendedConsensusError>>
                             + Send
                             + 'static,
            > + Clone
            + Send
            + 'static,
        impl tower::Service<
                VerifyTxRequest,
                Response = VerifyTxResponse,
                Error = ExtendedConsensusError,
                Future = impl Future<Output = Result<VerifyTxResponse, ExtendedConsensusError>>
                             + Send
                             + 'static,
            > + Clone
            + Send
            + 'static,
    ),
    ConsensusError,
>
where
    D: Database + Clone + Send + Sync + 'static,
    D::Future: Send + 'static,
    TxP: tower::Service<TxPoolRequest, Response = TxPoolResponse, Error = TxNotInPool>
        + Clone
        + Send
        + Sync
        + 'static,
    TxP::Future: Send + 'static,
    Ctx: tower::Service<
            BlockChainContextRequest,
            Response = BlockChainContextResponse,
            Error = tower::BoxError,
        > + Clone
        + Send
        + Sync
        + 'static,
    Ctx::Future: Send + 'static,
{
    let tx_svc = transactions::TxVerifierService::new(database);
    let block_svc = block::BlockVerifierService::new(ctx_svc, tx_svc.clone(), tx_pool);
    Ok((block_svc, tx_svc))
}

pub trait Database:
    tower::Service<DatabaseRequest, Response = DatabaseResponse, Error = tower::BoxError>
{
}

impl<T: tower::Service<DatabaseRequest, Response = DatabaseResponse, Error = tower::BoxError>>
    Database for T
{
}

#[derive(Debug, Copy, Clone)]
pub struct ExtendedBlockHeader {
    pub version: HardFork,
    pub vote: HardFork,

    pub timestamp: u64,
    pub cumulative_difficulty: u128,

    pub block_weight: usize,
    pub long_term_weight: usize,
}

#[derive(Debug, Clone)]
pub enum DatabaseRequest {
    BlockExtendedHeader(cuprate_common::BlockID),
    BlockHash(u64),

    BlockExtendedHeaderInRange(std::ops::Range<u64>),

    ChainHeight,
    GeneratedCoins,

    Outputs(HashMap<u64, HashSet<u64>>),
    NumberOutputsWithAmount(Vec<u64>),

    CheckKIsNotSpent(HashSet<[u8; 32]>),

    #[cfg(feature = "binaries")]
    BlockBatchInRange(std::ops::Range<u64>),
}

#[derive(Debug)]
pub enum DatabaseResponse {
    BlockExtendedHeader(ExtendedBlockHeader),
    BlockHash([u8; 32]),

    BlockExtendedHeaderInRange(Vec<ExtendedBlockHeader>),

    ChainHeight(u64, [u8; 32]),
    GeneratedCoins(u64),

    Outputs(HashMap<u64, HashMap<u64, OutputOnChain>>),
    NumberOutputsWithAmount(HashMap<u64, usize>),

    /// returns true if key images are spent
    CheckKIsNotSpent(bool),

    #[cfg(feature = "binaries")]
    BlockBatchInRange(
        Vec<(
            monero_serai::block::Block,
            Vec<monero_serai::transaction::Transaction>,
        )>,
    ),
}

#[derive(Debug, Copy, Clone, thiserror::Error)]
#[error("The transaction requested was not in the transaction pool")]
pub struct TxNotInPool;

pub enum TxPoolRequest {
    Transactions(Vec<[u8; 32]>),
}

pub enum TxPoolResponse {
    Transactions(Vec<Arc<transactions::TransactionVerificationData>>),
}

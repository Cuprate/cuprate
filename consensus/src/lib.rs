use std::{
    collections::{HashMap, HashSet},
    future::Future,
    sync::Arc,
};

mod batch_verifier;
pub mod block;
pub mod context;
pub mod genesis;
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
    BlockChainContextResponse, ContextConfig, HardFork,
};
pub use transactions::{VerifyTxRequest, VerifyTxResponse};

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
                Error = ConsensusError,
                Future = impl Future<Output = Result<VerifyBlockResponse, ConsensusError>>
                             + Send
                             + 'static,
            > + Clone
            + Send
            + 'static,
        impl tower::Service<
                VerifyTxRequest,
                Response = VerifyTxResponse,
                Error = ConsensusError,
                Future = impl Future<Output = Result<VerifyTxResponse, ConsensusError>> + Send + 'static,
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

// TODO: split this enum up.
#[derive(Debug, thiserror::Error)]
pub enum ConsensusError {
    #[error("Miner transaction invalid: {0}")]
    MinerTransaction(&'static str),
    #[error("Transaction sig invalid: {0}")]
    TransactionSignatureInvalid(&'static str),
    #[error("Transaction has too high output amount")]
    TransactionOutputsTooMuch,
    #[error("Transaction inputs overflow")]
    TransactionInputsOverflow,
    #[error("Transaction outputs overflow")]
    TransactionOutputsOverflow,
    #[error("Transaction has an invalid output: {0}")]
    TransactionInvalidOutput(&'static str),
    #[error("Transaction has an invalid version")]
    TransactionVersionInvalid,
    #[error("Transaction an invalid input: {0}")]
    TransactionHasInvalidInput(&'static str),
    #[error("Transaction has invalid ring: {0}")]
    TransactionHasInvalidRing(&'static str),
    #[error("Block has an invalid proof of work")]
    BlockPOWInvalid,
    #[error("Block has a timestamp outside of the valid range")]
    BlockTimestampInvalid,
    #[error("Block is too large")]
    BlockIsTooLarge,
    #[error("Invalid hard fork version: {0}")]
    InvalidHardForkVersion(&'static str),
    #[error("The block has a different previous hash than expected")]
    BlockIsNotApartOfChain,
    #[error("One or more transaction is not in the transaction pool")]
    TxNotInPool(#[from] TxNotInPool),
    #[error("Database error: {0}")]
    Database(#[from] tower::BoxError),
}

pub trait Database:
    tower::Service<DatabaseRequest, Response = DatabaseResponse, Error = tower::BoxError>
{
}

impl<T: tower::Service<DatabaseRequest, Response = DatabaseResponse, Error = tower::BoxError>>
    Database for T
{
}

#[derive(Debug)]
pub struct OutputOnChain {
    height: u64,
    time_lock: monero_serai::transaction::Timelock,
    key: curve25519_dalek::EdwardsPoint,
    mask: curve25519_dalek::EdwardsPoint,
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
    NumberOutputsWithAmount(u64),

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
    NumberOutputsWithAmount(usize),

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

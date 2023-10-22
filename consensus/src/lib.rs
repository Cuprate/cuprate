use std::collections::{HashMap, HashSet};

pub mod block;
pub mod context;
pub mod genesis;
mod helper;
pub mod miner_tx;
#[cfg(feature = "binaries")]
pub mod rpc;
pub mod transactions;

pub use block::VerifyBlockRequest;
pub use context::{ContextConfig, HardFork};
pub use transactions::VerifyTxRequest;

pub async fn initialize_verifier<D>(
    database: D,
    cfg: ContextConfig,
) -> Result<
    (
        impl tower::Service<VerifyBlockRequest, Response = (), Error = ConsensusError>,
        impl tower::Service<VerifyTxRequest, Response = (), Error = ConsensusError>,
    ),
    ConsensusError,
>
where
    D: Database + Clone + Send + Sync + 'static,
    D::Future: Send + 'static,
{
    let context_svc = context::initialize_blockchain_context(cfg, database.clone()).await?;
    let tx_svc = transactions::TxVerifierService::new(database);
    let block_svc = block::BlockVerifierService::new(context_svc, tx_svc.clone());
    Ok((block_svc, tx_svc))
}

#[derive(Debug, thiserror::Error)]
pub enum ConsensusError {
    #[error("Transaction sig invalid: {0}")]
    TransactionSignatureInvalid(&'static str),
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

    Outputs(HashMap<u64, HashSet<u64>>),
    NumberOutputsWithAmount(u64),

    #[cfg(feature = "binaries")]
    BlockBatchInRange(std::ops::Range<u64>),
}

#[derive(Debug)]
pub enum DatabaseResponse {
    BlockExtendedHeader(ExtendedBlockHeader),
    BlockHash([u8; 32]),

    BlockExtendedHeaderInRange(Vec<ExtendedBlockHeader>),

    ChainHeight(u64, [u8; 32]),

    Outputs(HashMap<u64, HashMap<u64, OutputOnChain>>),
    NumberOutputsWithAmount(usize),

    #[cfg(feature = "binaries")]
    BlockBatchInRange(
        Vec<(
            monero_serai::block::Block,
            Vec<monero_serai::transaction::Transaction>,
        )>,
    ),
}

//! Cuprate Consensus
//!
//! This crate contains 3 [`tower::Service`]s that implement Monero's consensus rules:
//!
//! - [`BlockChainContextService`] Which handles keeping the current state of the blockchain.
//! - [`BlockVerifierService`] Which handles block verification.
//! - [`TxVerifierService`] Which handles transaction verification.
//!
//! This crate is generic over the database which is implemented as a [`tower::Service`]. To
//! implement a database you need to have a service which accepts [`DatabaseRequest`] and responds
//! with [`DatabaseResponse`].
//!
use std::{
    collections::{HashMap, HashSet},
    future::Future,
};

use cuprate_consensus_rules::{transactions::OutputOnChain, ConsensusError, HardFork};

mod batch_verifier;
pub mod block;
pub mod context;
#[cfg(test)]
mod tests;
pub mod transactions;

pub use block::{
    BlockVerifierService, PrePreparedBlock, VerifiedBlockInformation, VerifyBlockRequest,
    VerifyBlockResponse,
};
pub use context::{
    initialize_blockchain_context, BlockChainContext, BlockChainContextRequest,
    BlockChainContextResponse, BlockChainContextService, ContextConfig,
};
pub use transactions::{TxVerifierService, VerifyTxRequest, VerifyTxResponse};

/// An Error returned from one of the consensus services.
#[derive(Debug, thiserror::Error)]
pub enum ExtendedConsensusError {
    /// A consensus error.
    #[error("{0}")]
    ConErr(#[from] ConsensusError),
    /// A database error.
    #[error("Database error: {0}")]
    DBErr(#[from] tower::BoxError),
    /// The transactions passed in with this block were not the ones needed.
    #[error("The transactions passed in with the block are incorrect.")]
    TxsIncludedWithBlockIncorrect,
    /// One or more statements in the batch verifier was invalid.
    #[error("One or more statements in the batch verifier was invalid.")]
    OneOrMoreBatchVerificationStatementsInvalid,
}

/// Initialize the 2 verifier [`tower::Service`]s (block and transaction).
pub async fn initialize_verifier<D, Ctx>(
    database: D,
    ctx_svc: Ctx,
) -> Result<
    (
        BlockVerifierService<Ctx, TxVerifierService<D>, D>,
        TxVerifierService<D>,
    ),
    ConsensusError,
>
where
    D: Database + Clone + Send + Sync + 'static,
    D::Future: Send + 'static,
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
    let tx_svc = TxVerifierService::new(database.clone());
    let block_svc = BlockVerifierService::new(ctx_svc, tx_svc.clone(), database);
    Ok((block_svc, tx_svc))
}

/// An internal trait used to represent a database so we don't have to write [`tower::Service`] bounds
/// everywhere.
pub trait Database:
    tower::Service<
    DatabaseRequest,
    Response = DatabaseResponse,
    Error = tower::BoxError,
    Future = Self::Future2,
>
{
    type Future2: Future<Output = Result<Self::Response, Self::Error>> + Send + 'static;
}

impl<T: tower::Service<DatabaseRequest, Response = DatabaseResponse, Error = tower::BoxError>>
    Database for T
where
    T::Future: Future<Output = Result<Self::Response, Self::Error>> + Send + 'static,
{
    type Future2 = T::Future;
}

/// An extended block header.
#[derive(Debug, Copy, Clone)]
pub struct ExtendedBlockHeader {
    /// The blocks major version.
    pub version: HardFork,
    /// The blocks vote.
    pub vote: HardFork,

    /// The blocks timestamp.
    pub timestamp: u64,
    /// The blocks cumulative difficulty.
    pub cumulative_difficulty: u128,

    /// The blocks weight.
    pub block_weight: usize,
    /// The blocks long term weight.
    pub long_term_weight: usize,
}

/// A database request to the database [`tower::Service`]
#[derive(Debug, Clone)]
pub enum DatabaseRequest {
    /// A block extended header request.
    /// Must return: [`DatabaseResponse::BlockExtendedHeader`]
    BlockExtendedHeader(u64),
    /// A block hash request.
    /// Must return: [`DatabaseResponse::BlockHash`]     
    BlockHash(u64),

    /// Removes the block hashes that are not in the _main_ chain.
    ///
    /// This should filter (remove) hashes in alt-blocks as well.
    FilterUnknownHashes(HashSet<[u8; 32]>),

    /// A request for multiple block extended headers.
    /// Must return: [`DatabaseResponse::BlockExtendedHeaderInRange`]
    BlockExtendedHeaderInRange(std::ops::Range<u64>),

    /// A request for the chains height.
    /// Must return: [`DatabaseResponse::ChainHeight`]
    ChainHeight,
    /// A request for the total amount of generated coins.
    /// Must return: [`DatabaseResponse::GeneratedCoins`]
    GeneratedCoins,

    /// A request for transaction outputs, this contains a map of amounts to amount indexes.
    /// Must return: [`DatabaseResponse::Outputs`]
    Outputs(HashMap<u64, HashSet<u64>>),
    /// A request for the number of outputs with these amounts.
    /// Must return: [`DatabaseResponse::NumberOutputsWithAmount`]     
    NumberOutputsWithAmount(Vec<u64>),

    /// A request to check if these key images are in the database.
    /// Must return: [`DatabaseResponse::KeyImagesSpent`]     
    KeyImagesSpent(HashSet<[u8; 32]>),
}

#[derive(Debug)]
pub enum DatabaseResponse {
    /// A block extended header response.
    BlockExtendedHeader(ExtendedBlockHeader),
    /// A block hash response.
    BlockHash([u8; 32]),

    FilteredHashes(HashSet<[u8; 32]>),

    /// A batch block extended header response.
    BlockExtendedHeaderInRange(Vec<ExtendedBlockHeader>),

    /// A chain height response.
    /// Should contains the chains height and top block hash.
    ChainHeight(u64, [u8; 32]),
    /// Generated coins response.
    /// Should contain the total amount of coins emitted in all block rewards.
    GeneratedCoins(u64),

    /// Outputs response.
    /// Should contain a map of (amounts, amount_idx) -> Output.
    /// If an outputs requested does not exist this should *not* be an error, the output
    /// should just be omitted from the map.
    Outputs(HashMap<u64, HashMap<u64, OutputOnChain>>),
    /// Number of outputs response.
    /// Should contain a map of amounts -> numb outs.
    /// If there are no outputs with that amount then the numb outs should be zero, *no* amounts
    /// requested should be omitted.
    NumberOutputsWithAmount(HashMap<u64, usize>),

    /// Key images spent response.
    /// returns true if key images are spent
    KeyImagesSpent(bool),
}

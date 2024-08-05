//! Cuprate Consensus
//!
//! This crate contains 3 [`tower::Service`]s that implement Monero's consensus rules:
//!
//! - [`BlockChainContextService`] Which handles keeping the current state of the blockchain.
//! - [`BlockVerifierService`] Which handles block verification.
//! - [`TxVerifierService`] Which handles transaction verification.
//!
//! This crate is generic over the database which is implemented as a [`tower::Service`]. To
//! implement a database you need to have a service which accepts [`BlockchainReadRequest`] and responds
//! with [`BlockchainResponse`].
//!
use cuprate_consensus_rules::{ConsensusError, HardFork};

mod batch_verifier;
pub mod block;
pub mod context;
#[cfg(test)]
mod tests;
pub mod transactions;

pub use block::{BlockVerifierService, VerifyBlockRequest, VerifyBlockResponse};
pub use context::{
    initialize_blockchain_context, BlockChainContext, BlockChainContextRequest,
    BlockChainContextResponse, BlockChainContextService, ContextConfig,
};
pub use transactions::{TxVerifierService, VerifyTxRequest, VerifyTxResponse};

// re-export.
pub use cuprate_types::blockchain::{BlockchainReadRequest, BlockchainResponse};

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
    /// A request to verify a batch of blocks had no blocks in the batch.
    #[error("A request to verify a batch of blocks had no blocks in the batch.")]
    NoBlocksToVerify,
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

use __private::Database;

pub mod __private {
    use std::future::Future;

    use cuprate_types::blockchain::{BlockchainReadRequest, BlockchainResponse};

    /// A type alias trait used to represent a database, so we don't have to write [`tower::Service`] bounds
    /// everywhere.
    ///
    /// Automatically implemented for:
    /// ```ignore
    /// tower::Service<BCReadRequest, Response = BCResponse, Error = tower::BoxError>
    /// ```
    pub trait Database:
        tower::Service<
            BlockchainReadRequest,
        Response =BlockchainResponse,
        Error = tower::BoxError,
        Future = Self::Future2,
    >
    {
        type Future2: Future<Output = Result<Self::Response, Self::Error>> + Send + 'static;
    }

    impl<T: tower::Service<BlockchainReadRequest, Response =BlockchainResponse, Error = tower::BoxError>>
        crate::Database for T
    where
        T::Future: Future<Output = Result<Self::Response, Self::Error>> + Send + 'static,
    {
        type Future2 = T::Future;
    }
}

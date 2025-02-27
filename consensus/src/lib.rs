//! Cuprate Consensus
//!
//! This crate contains Monero [`block`] and [`transactions`] verification functionality.
//!
//! This crate is generic over the database which is implemented as a [`tower::Service`]. To
//! implement a database you need to have a service which accepts [`BlockchainReadRequest`] and responds
//! with [`BlockchainResponse`].
//!

#![forbid(
    unsafe_code,
    missing_copy_implementations,
    reason = "Crate-specific lints. There should be good reasoning when removing these."
)]

cfg_if::cfg_if! {
    // Used in external `tests/`.
    if #[cfg(test)] {
        use cuprate_test_utils as _;
        use curve25519_dalek as _;
        use hex_literal as _;
        use futures as _;
    }
}

use cuprate_consensus_rules::ConsensusError;

pub mod batch_verifier;
pub mod block;
#[cfg(test)]
mod tests;
pub mod transactions;

pub use cuprate_consensus_context::{
    initialize_blockchain_context, BlockChainContextRequest, BlockChainContextResponse,
    BlockchainContext, BlockchainContextService, ContextConfig,
};

// re-export.
pub use cuprate_consensus_rules::genesis::generate_genesis_block;
pub use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    HardFork,
};

/// An Error returned from one of the consensus services.
#[derive(Debug, thiserror::Error)]
#[expect(variant_size_differences)]
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

use __private::Database;

pub mod __private {
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
        Response = BlockchainResponse,
        Error = tower::BoxError,
        Future: Send + 'static,
    >
    {
    }

    impl<
            T: tower::Service<
                BlockchainReadRequest,
                Response = BlockchainResponse,
                Error = tower::BoxError,
                Future: Send + 'static,
            >,
        > Database for T
    {
    }
}

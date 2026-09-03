//! Cuprate Consensus
//!
//! This crate contains Monero [`block`] and [`transactions`] verification functionality.
//!
//! This crate is generic over the database which is implemented as a [`tower::Service`]. To
//! implement a database you need to have a service which accepts [`BlockchainReadRequest`] and responds
//! with [`BlockchainResponse`].
//!

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

/// The verification context to verify a block or transaction with.
#[expect(clippy::large_enum_variant)]
pub enum VerificationContext<D> {
    /// A full database
    Database(D),
    /// A batch prepared cache.
    ///
    /// This cache is only valid for the set of blocks it was created with, it should not be used for other blocks.
    /// You must pass blocks in sequentially.
    BatchPrepareCache(block::BatchPrepareCache),
}

/// An Error returned from one of the consensus services.
#[derive(Debug, thiserror::Error)]
pub enum ExtendedConsensusError {
    /// A consensus error.
    #[error("{0}")]
    ConsensusError(#[from] ConsensusError),
    /// A service error that we cannot recover from.
    ///
    /// If this happens, no more consensus verification should be done with the given inner services.
    #[error("Fatal error: {0}")]
    FatalError(#[from] tower::BoxError),
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

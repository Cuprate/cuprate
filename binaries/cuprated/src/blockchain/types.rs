use tower::util::MapErr;

use cuprate_blockchain::{service::BlockchainReadHandle, BlockchainError};

/// The [`BlockchainReadHandle`] with the [`tower::Service::Error`] mapped to conform to what the consensus crate requires.
pub type ConsensusBlockchainReadHandle =
    MapErr<BlockchainReadHandle, fn(BlockchainError) -> tower::BoxError>;

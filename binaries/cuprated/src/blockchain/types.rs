use tower::util::MapErr;

use cuprate_blockchain::{cuprate_database::RuntimeError, service::BlockchainReadHandle};

/// The [`BlockchainReadHandle`] with the [`tower::Service::Error`] mapped to conform to what the consensus crate requires.
pub type ConsensusBlockchainReadHandle =
    MapErr<BlockchainReadHandle, fn(RuntimeError) -> tower::BoxError>;

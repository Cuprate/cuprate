use tower::util::MapErr;

use cuprate_blockchain::{cuprate_database::RuntimeError, service::BlockchainReadHandle};
use cuprate_consensus::{BlockChainContextService, BlockVerifierService, TxVerifierService};

/// The [`BlockVerifierService`] with all generic types defined.
pub type ConcreteBlockVerifierService = BlockVerifierService<
    BlockChainContextService,
    ConcreteTxVerifierService,
    ConsensusBlockchainReadHandle,
>;

/// The [`TxVerifierService`] with all generic types defined.
pub type ConcreteTxVerifierService = TxVerifierService<ConsensusBlockchainReadHandle>;

/// The [`BlockchainReadHandle`] with the [`tower::Service::Error`] mapped to conform to what the consensus crate requires.
pub type ConsensusBlockchainReadHandle =
    MapErr<BlockchainReadHandle, fn(RuntimeError) -> tower::BoxError>;

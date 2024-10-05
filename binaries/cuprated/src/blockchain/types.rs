use std::task::{Context, Poll};

use futures::future::BoxFuture;
use futures::{FutureExt, TryFutureExt};
use tower::{util::MapErr, Service};

use cuprate_blockchain::{cuprate_database::RuntimeError, service::BlockchainReadHandle};
use cuprate_consensus::{BlockChainContextService, BlockVerifierService, TxVerifierService};
use cuprate_p2p::block_downloader::{ChainSvcRequest, ChainSvcResponse};
use cuprate_types::blockchain::{BlockchainReadRequest, BlockchainResponse};

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

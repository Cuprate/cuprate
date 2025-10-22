use futures::future::{MapErr, TryFutureExt};
use std::task::{Context, Poll};
use tower::Service;

use cuprate_blockchain::{
    cuprate_database::ConcreteEnv, cuprate_database::RuntimeError, BlockchainDatabaseService,
};
use cuprate_types::blockchain::{
    BlockchainReadRequest, BlockchainResponse, BlockchainWriteRequest,
};

/// The [`BlockchainReadHandle`] with the [`tower::Service::Error`] mapped to conform to what the consensus crate requires.
#[derive(Clone)]
pub struct ConsensusBlockchainReadHandle(pub BlockchainDatabaseService<ConcreteEnv>);

impl Service<BlockchainReadRequest> for ConsensusBlockchainReadHandle {
    type Response = BlockchainResponse;
    type Error = tower::BoxError;
    type Future = MapErr<
        <BlockchainDatabaseService<ConcreteEnv> as Service<BlockchainReadRequest>>::Future,
        fn(RuntimeError) -> tower::BoxError,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Service::<BlockchainReadRequest>::poll_ready(&mut self.0, cx).map_err(Into::into)
    }

    fn call(&mut self, req: BlockchainReadRequest) -> Self::Future {
        Service::<BlockchainReadRequest>::call(&mut self.0, req).map_err(Into::into)
    }
}

#[derive(Clone)]
pub struct BlockchainReadHandle(pub BlockchainDatabaseService<ConcreteEnv>);

impl Service<BlockchainReadRequest> for BlockchainReadHandle {
    type Response = BlockchainResponse;
    type Error = RuntimeError;
    type Future =
        <BlockchainDatabaseService<ConcreteEnv> as Service<BlockchainReadRequest>>::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Service::<BlockchainReadRequest>::poll_ready(&mut self.0, cx)
    }

    fn call(&mut self, req: BlockchainReadRequest) -> Self::Future {
        Service::<BlockchainReadRequest>::call(&mut self.0, req)
    }
}

#[derive(Clone)]
pub struct BlockchainWriteHandle(pub BlockchainDatabaseService<ConcreteEnv>);

impl Service<BlockchainWriteRequest> for BlockchainWriteHandle {
    type Response = BlockchainResponse;
    type Error = RuntimeError;
    type Future =
        <BlockchainDatabaseService<ConcreteEnv> as Service<BlockchainWriteRequest>>::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Service::<BlockchainWriteRequest>::poll_ready(&mut self.0, cx)
    }

    fn call(&mut self, req: BlockchainWriteRequest) -> Self::Future {
        Service::<BlockchainWriteRequest>::call(&mut self.0, req)
    }
}

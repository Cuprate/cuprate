use cuprate_blockchain::cuprate_database::RuntimeError;
use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_types::blockchain::{BlockchainReadRequest, BlockchainResponse};
use futures::future::MapErr;
use futures::TryFutureExt;
use std::task::{Context, Poll};
use tower::Service;

#[derive(Clone)]
pub struct ConsensusBlockchainReadHandle(BlockchainReadHandle);

impl Service<BlockchainReadRequest> for ConsensusBlockchainReadHandle {
    type Response = BlockchainResponse;
    type Error = tower::BoxError;
    type Future = MapErr<
        <BlockchainReadHandle as Service<BlockchainReadRequest>>::Future,
        fn(RuntimeError) -> tower::BoxError,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: BlockchainReadRequest) -> Self::Future {
        self.0.call(req).map_err(Into::into)
    }
}

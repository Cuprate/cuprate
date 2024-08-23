use cuprate_p2p_core::{ProtocolRequest, ProtocolResponse};
use futures::future::BoxFuture;
use futures::FutureExt;
use std::task::{Context, Poll};
use tower::Service;
use tracing::trace;

#[derive(Clone)]
pub struct P2pProtocolRequestHandler;

impl Service<ProtocolRequest> for P2pProtocolRequestHandler {
    type Response = ProtocolResponse;
    type Error = tower::BoxError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: ProtocolRequest) -> Self::Future {
        match req {
            ProtocolRequest::GetObjects(_) => trace!("TODO: GetObjects"),
            ProtocolRequest::GetChain(_) => trace!("TODO: GetChain"),
            ProtocolRequest::FluffyMissingTxs(_) => trace!("TODO: FluffyMissingTxs"),
            ProtocolRequest::GetTxPoolCompliment(_) => trace!("TODO: GetTxPoolCompliment"),
            ProtocolRequest::NewBlock(_) => trace!("TODO: NewBlock"),
            ProtocolRequest::NewFluffyBlock(_) => trace!("TODO: NewFluffyBlock"),
            ProtocolRequest::NewTransactions(_) => trace!("TODO: NewTransactions"),
        }

        async { Ok(ProtocolResponse::NA) }.boxed()
    }
}

use futures::lock::{OwnedMutexGuard, OwnedMutexLockFuture};
use futures::{FutureExt, TryFutureExt};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use monero_serai::rpc::{HttpRpc, RpcConnection};

use cuprate_common::BlockID;

use crate::{DatabaseRequest, DatabaseResponse};

enum RpcState<R: RpcConnection> {
    Locked,
    Acquiring(OwnedMutexLockFuture<monero_serai::rpc::Rpc<R>>),
    Acquired(OwnedMutexGuard<monero_serai::rpc::Rpc<R>>),
}
pub struct Rpc<R: RpcConnection>(
    Arc<futures::lock::Mutex<monero_serai::rpc::Rpc<R>>>,
    RpcState<R>,
);

impl Rpc<HttpRpc> {
    pub fn new_http(addr: String) -> Rpc<HttpRpc> {
        let http_rpc = HttpRpc::new(addr).unwrap();
        Rpc(
            Arc::new(futures::lock::Mutex::new(http_rpc)),
            RpcState::Locked,
        )
    }
}

impl<R: RpcConnection> Clone for Rpc<R> {
    fn clone(&self) -> Self {
        Rpc(Arc::clone(&self.0), RpcState::Locked)
    }
}

impl<R: RpcConnection + Send + Sync + 'static> tower::Service<DatabaseRequest> for Rpc<R> {
    type Response = DatabaseResponse;
    type Error = tower::BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        loop {
            match &mut self.1 {
                RpcState::Locked => self.1 = RpcState::Acquiring(self.0.clone().lock_owned()),
                RpcState::Acquiring(rpc) => {
                    self.1 = RpcState::Acquired(futures::ready!(rpc.poll_unpin(cx)))
                }
                RpcState::Acquired(_) => return Poll::Ready(Ok(())),
            }
        }
    }

    fn call(&mut self, req: DatabaseRequest) -> Self::Future {
        let RpcState::Acquired(rpc) = std::mem::replace(&mut self.1, RpcState::Locked) else {
            panic!("poll_ready was not called first!");
        };

        match req {
            DatabaseRequest::ChainHeight => async move {
                rpc.get_height()
                    .map_ok(|height| DatabaseResponse::ChainHeight(height.try_into().unwrap()))
                    .map_err(Into::into)
                    .await
            }
            .boxed(),

            DatabaseRequest::BlockHeader(id) => match id {
                BlockID::Hash(hash) => async move {
                    rpc.get_block(hash)
                        .map_ok(|block| DatabaseResponse::BlockHeader(block.header))
                        .map_err(Into::into)
                        .await
                }
                .boxed(),
                BlockID::Height(height) => async move {
                    rpc.get_block_by_number(height.try_into().unwrap())
                        .map_ok(|block| DatabaseResponse::BlockHeader(block.header))
                        .map_err(Into::into)
                        .await
                }
                .boxed(),
            },
        }
    }
}

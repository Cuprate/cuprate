use crate::txpool::dandelion::{DandelionTx, TxId};
use bytes::Bytes;
use cuprate_dandelion_tower::traits::{TxStoreRequest, TxStoreResponse};
use cuprate_database::RuntimeError;
use cuprate_txpool::service::interface::{TxpoolReadRequest, TxpoolReadResponse};
use cuprate_txpool::service::{TxpoolReadHandle, TxpoolWriteHandle};
use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt, TryFutureExt};
use std::task::{Context, Poll};
use tower::util::Oneshot;
use tower::{Service, ServiceExt};

pub struct TxStoreService {
    txpool_read_handle: TxpoolReadHandle,
    txpool_write_handle: TxpoolWriteHandle,
}

impl Service<TxStoreRequest<TxId>> for TxStoreService {
    type Response = TxStoreResponse<DandelionTx>;
    type Error = tower::BoxError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: TxStoreRequest<TxId>) -> Self::Future {
        match req {
            TxStoreRequest::Get(tx_id) => self
                .txpool_read_handle
                .clone()
                .oneshot(TxpoolReadRequest::TxBlob(tx_id))
                .map(|res| match res {
                    Ok(TxpoolReadResponse::TxBlob(blob)) => Ok(TxStoreResponse::Transaction(Some(
                        (DandelionTx(Bytes::from(blob)), todo!()),
                    ))),
                    Err(RuntimeError::KeyNotFound) => Ok(TxStoreResponse::Transaction(None)),
                    Err(e) => Err(e.into()),
                    Ok(_) => unreachable!(),
                })
                .boxed(),
            TxStoreRequest::Promote(tx_id) => {
                todo!()
            }
        }
    }
}

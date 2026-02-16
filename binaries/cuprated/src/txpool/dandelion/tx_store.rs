use std::{
    future::ready,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::{future::BoxFuture, FutureExt};
use tokio::sync::mpsc;
use tower::{Service, ServiceExt};

use super::{DandelionTx, TxId};
use cuprate_dandelion_tower::{
    traits::{TxStoreRequest, TxStoreResponse},
    State,
};
use cuprate_txpool::service::{
    interface::{TxpoolReadRequest, TxpoolReadResponse, TxpoolWriteRequest},
    TxpoolReadHandle, TxpoolWriteHandle,
};
use cuprate_txpool::TxPoolError;

/// The dandelion tx-store service.
///
/// This is just mapping the interface [`cuprate_dandelion_tower`] wants to what [`cuprate_txpool`] provides.
pub struct TxStoreService {
    pub txpool_read_handle: TxpoolReadHandle,
    pub promote_tx: mpsc::UnboundedSender<[u8; 32]>,
}

impl Service<TxStoreRequest<TxId>> for TxStoreService {
    type Response = TxStoreResponse<DandelionTx>;
    type Error = tower::BoxError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: TxStoreRequest<TxId>) -> Self::Future {
        match req {
            TxStoreRequest::Get(tx_id) => self
                .txpool_read_handle
                .clone()
                .oneshot(TxpoolReadRequest::TxBlob(tx_id))
                .map(|res| match res {
                    Ok(TxpoolReadResponse::TxBlob {
                        tx_blob,
                        state_stem,
                    }) => {
                        let state = if state_stem {
                            State::Stem
                        } else {
                            State::Fluff
                        };

                        Ok(TxStoreResponse::Transaction(Some((
                            DandelionTx(Bytes::from(tx_blob)),
                            state,
                        ))))
                    }
                    Err(TxPoolError::NotFound) => Ok(TxStoreResponse::Transaction(None)),
                    Err(e) => Err(e.into()),
                    Ok(_) => unreachable!(),
                })
                .boxed(),
            TxStoreRequest::Promote(tx_id) => ready(
                self.promote_tx
                    .send(tx_id)
                    .map_err(Into::into)
                    .map(|()| TxStoreResponse::Ok),
            )
            .boxed(),
        }
    }
}

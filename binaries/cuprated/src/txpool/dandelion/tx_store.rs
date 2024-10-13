use std::{
    f32::consts::E,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::{future::BoxFuture, FutureExt, StreamExt, TryFutureExt};
use tower::{util::Oneshot, Service, ServiceExt};

use cuprate_dandelion_tower::{
    traits::{TxStoreRequest, TxStoreResponse},
    State,
};
use cuprate_database::RuntimeError;
use cuprate_txpool::service::{
    interface::{TxpoolReadRequest, TxpoolReadResponse, TxpoolWriteRequest, TxpoolWriteResponse},
    TxpoolReadHandle, TxpoolWriteHandle,
};

use super::{DandelionTx, TxId};

/// The dandelion tx-store service.
///
/// This is just mapping the interface [`cuprate_dandelion_tower`] wants to what [`cuprate_txpool`] provides.
pub struct TxStoreService {
    pub txpool_read_handle: TxpoolReadHandle,
    pub txpool_write_handle: TxpoolWriteHandle,
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
                    Err(RuntimeError::KeyNotFound) => Ok(TxStoreResponse::Transaction(None)),
                    Err(e) => Err(e.into()),
                    Ok(_) => unreachable!(),
                })
                .boxed(),
            TxStoreRequest::Promote(tx_id) => self
                .txpool_write_handle
                .oneshot(TxpoolWriteRequest::Promote(tx_id))
                .map(|res| match res {
                    Ok(_) | Err(RuntimeError::KeyNotFound) => TxStoreResponse::Ok,
                    Err(e) => Err(e.into()),
                })
                .boxed(),
        }
    }
}

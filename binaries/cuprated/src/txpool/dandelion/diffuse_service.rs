use std::{
    future::{ready, Ready},
    task::{Context, Poll},
};

use futures::FutureExt;
use tower::Service;

use cuprate_dandelion_tower::traits::DiffuseRequest;
use cuprate_p2p::{BroadcastRequest, BroadcastSvc};
use cuprate_p2p_core::{ClearNet, NetworkZone};

use crate::txpool::dandelion::DandelionTx;

/// The dandelion diffusion service.
pub struct DiffuseService<N: NetworkZone> {
    pub clear_net_broadcast_service: BroadcastSvc<N>,
}

impl<N: NetworkZone> Service<DiffuseRequest<DandelionTx>> for DiffuseService<N> {
    type Response = ();
    type Error = tower::BoxError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.clear_net_broadcast_service
            .poll_ready(cx)
            .map_err(Into::into)
    }

    fn call(&mut self, req: DiffuseRequest<DandelionTx>) -> Self::Future {
        // TODO: the dandelion crate should pass along where we got the tx from.
        let Ok(()) = self
            .clear_net_broadcast_service
            .call(BroadcastRequest::Transaction {
                tx_bytes: req.0 .0,
                direction: None,
                received_from: None,
            })
            .into_inner();

        ready(Ok(()))
    }
}

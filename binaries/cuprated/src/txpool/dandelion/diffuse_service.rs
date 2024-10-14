use std::{
    future::{ready, Ready},
    task::{Context, Poll},
};

use futures::FutureExt;
use tower::Service;

use cuprate_dandelion_tower::traits::DiffuseRequest;
use cuprate_p2p::{BroadcastRequest, BroadcastSvc, NetworkInterface};
use cuprate_p2p_core::ClearNet;

use super::DandelionTx;

/// The dandelion diffusion service.
pub struct DiffuseService {
    pub clear_net_broadcast_service: BroadcastSvc<ClearNet>,
}

impl Service<DiffuseRequest<DandelionTx>> for DiffuseService {
    type Response = ();
    type Error = tower::BoxError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.clear_net_broadcast_service
            .poll_ready(cx)
            .map_err(Into::into)
    }

    fn call(&mut self, req: DiffuseRequest<DandelionTx>) -> Self::Future {
        // TODO: Call `into_inner` when 1.82.0 stabilizes
        self.clear_net_broadcast_service
            .call(BroadcastRequest::Transaction {
                tx_bytes: req.0 .0,
                direction: None,
                received_from: None,
            })
            .now_or_never()
            .unwrap()
            .expect("Broadcast service is Infallible");

        ready(Ok(()))
    }
}

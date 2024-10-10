use std::task::{Context, Poll};
use tower::Service;

use crate::txpool::dandelion::DandelionTx;
use cuprate_dandelion_tower::traits::DiffuseRequest;
use cuprate_p2p::{BroadcastRequest, BroadcastSvc, NetworkInterface};
use cuprate_p2p_core::ClearNet;

pub struct DiffuseService {
    pub clear_net_broadcast_service: BroadcastSvc<ClearNet>,
}

impl Service<DiffuseRequest<DandelionTx>> for DiffuseService {
    type Response = BroadcastSvc::Response;
    type Error = tower::BoxError;
    type Future = BroadcastSvc::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.clear_net_broadcast_service
            .poll_ready(cx)
            .map_err(Into::into)
    }

    fn call(&mut self, req: DiffuseRequest<DandelionTx>) -> Self::Future {
        self.clear_net_broadcast_service
            .call(BroadcastRequest::Transaction {
                tx_bytes: req.0 .0,
                direction: None,
                received_from: None,
            })
    }
}

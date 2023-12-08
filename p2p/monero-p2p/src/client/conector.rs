use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use futures::FutureExt;
use tower::{Service, ServiceExt};

use crate::{
    client::{DoHandshakeRequest, HandShaker, HandshakeError},
    AddressBook, ConnectionDirection, CoreSyncSvc, NetworkZone, PeerRequestHandler,
};

pub struct ConnectRequest<Z: NetworkZone> {
    pub addr: Z::Addr,
}

pub struct Connector<Z: NetworkZone, AdrBook, CSync, ReqHdlr> {
    handshaker: HandShaker<Z, AdrBook, CSync, ReqHdlr>,
}

impl<Z: NetworkZone, AdrBook, CSync, ReqHdlr> Connector<Z, AdrBook, CSync, ReqHdlr> {
    pub fn new(handshaker: HandShaker<Z, AdrBook, CSync, ReqHdlr>) -> Self {
        Self { handshaker }
    }
}

impl<Z: NetworkZone, AdrBook, CSync, ReqHdlr> Service<ConnectRequest<Z>>
    for Connector<Z, AdrBook, CSync, ReqHdlr>
where
    AdrBook: AddressBook<Z> + Clone,
    CSync: CoreSyncSvc + Clone,
    ReqHdlr: PeerRequestHandler + Clone,
{
    type Response = ();
    type Error = HandshakeError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: ConnectRequest<Z>) -> Self::Future {
        tracing::debug!("Connecting to peer: {}", req.addr);
        let mut handshaker = self.handshaker.clone();

        async move {
            let (peer_stream, peer_sink) = Z::connect_to_peer(req.addr).await?;
            let req = DoHandshakeRequest {
                addr: req.addr,
                peer_stream,
                peer_sink,
                direction: ConnectionDirection::OutBound,
            };
            handshaker.ready().await?.call(req).await
        }
        .boxed()
    }
}

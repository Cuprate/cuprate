//! Connector
//!
//! This module handles connecting to peers and giving the sink/stream to the handshaker which will then
//! perform a handshake and create a [`Client`].
//!
//! This is where outbound connections are created.
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{FutureExt, Stream};
use tokio::sync::OwnedSemaphorePermit;
use tower::{Service, ServiceExt};

use crate::{
    client::{handshaker::HandShaker, Client, DoHandshakeRequest, HandshakeError, InternalPeerID},
    AddressBook, BroadcastMessage, ConnectionDirection, CoreSyncSvc, NetworkZone,
    ProtocolRequestHandlerMaker, Transport,
};

/// A request to connect to a peer.
pub struct ConnectRequest<Z: NetworkZone> {
    /// The peer's address.
    pub addr: Z::Addr,
    /// A permit which will be held be the connection allowing you to set limits on the number of
    /// connections.
    ///
    /// This doesn't have to be set.
    pub permit: Option<OwnedSemaphorePermit>,
}

/// The connector service, this service connects to peer and returns the [`Client`].
#[derive(Clone)]
pub struct Connector<Z: NetworkZone, T: Transport<Z>, AdrBook, CSync, ProtoHdlrMkr, BrdcstStrmMkr> {
    handshaker: HandShaker<Z, T, AdrBook, CSync, ProtoHdlrMkr, BrdcstStrmMkr>,
}

impl<Z: NetworkZone, T: Transport<Z>, AdrBook, CSync, ProtoHdlrMkr, BrdcstStrmMkr>
    Connector<Z, T, AdrBook, CSync, ProtoHdlrMkr, BrdcstStrmMkr>
{
    /// Create a new connector from a handshaker.
    pub const fn new(
        handshaker: HandShaker<Z, T, AdrBook, CSync, ProtoHdlrMkr, BrdcstStrmMkr>,
    ) -> Self {
        Self { handshaker }
    }
}

impl<Z: NetworkZone, T, AdrBook, CSync, ProtoHdlrMkr, BrdcstStrmMkr, BrdcstStrm>
    Service<ConnectRequest<Z>> for Connector<Z, T, AdrBook, CSync, ProtoHdlrMkr, BrdcstStrmMkr>
where
    T: Transport<Z>,
    AdrBook: AddressBook<Z> + Clone,
    CSync: CoreSyncSvc + Clone,
    ProtoHdlrMkr: ProtocolRequestHandlerMaker<Z> + Clone,
    BrdcstStrm: Stream<Item = BroadcastMessage> + Send + 'static,
    BrdcstStrmMkr: Fn(InternalPeerID<Z::Addr>) -> BrdcstStrm + Clone + Send + 'static,
{
    type Response = Client<Z>;
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
            let (peer_stream, peer_sink) =
                T::connect_to_peer(req.addr, handshaker.transport_config()).await?;
            let req = DoHandshakeRequest {
                addr: InternalPeerID::KnownAddr(req.addr),
                permit: req.permit,
                peer_stream,
                peer_sink,
                direction: ConnectionDirection::Outbound,
            };
            handshaker.ready().await?.call(req).await
        }
        .boxed()
    }
}

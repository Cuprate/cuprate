//! Wrapper around handshake logic that also opens a TCP connection.

use std::{
    future::Future,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{AsyncRead, AsyncWrite, FutureExt};
use monero_wire::{network_address::NetZone, NetworkAddress};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tower::{BoxError, Service, ServiceExt};
use tracing::Instrument;

use crate::peer::handshaker::ConnectionAddr;
use crate::{
    address_book::{AddressBookRequest, AddressBookResponse},
    connection_tracker::ConnectionTracker,
    protocol::{
        CoreSyncDataRequest, CoreSyncDataResponse, InternalMessageRequest, InternalMessageResponse,
    },
};

use super::{
    handshaker::{DoHandshakeRequest, Handshaker},
    Client,
};

async fn connect(addr: &NetworkAddress) -> Result<(impl AsyncRead, impl AsyncWrite), BoxError> {
    match addr.get_zone() {
        NetZone::Public => {
            let stream =
                tokio::net::TcpStream::connect(SocketAddr::try_from(*addr).unwrap()).await?;
            let (read, write) = stream.into_split();
            Ok((read.compat(), write.compat_write()))
        }
        _ => unimplemented!(),
    }
}

/// A wrapper around [`Handshake`] that opens a connection before
/// forwarding to the inner handshake service. Writing this as its own
/// [`tower::Service`] lets us apply unified timeout policies, etc.
#[derive(Debug, Clone)]
pub struct Connector<Svc, CoreSync, AdrBook>
where
    CoreSync: Service<CoreSyncDataRequest, Response = CoreSyncDataResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    CoreSync::Future: Send,

    Svc: Service<InternalMessageRequest, Response = InternalMessageResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    Svc::Future: Send,

    AdrBook: Service<AddressBookRequest, Response = AddressBookResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    AdrBook::Future: Send,
{
    handshaker: Handshaker<Svc, CoreSync, AdrBook>,
}

impl<Svc, CoreSync, AdrBook> Connector<Svc, CoreSync, AdrBook>
where
    CoreSync: Service<CoreSyncDataRequest, Response = CoreSyncDataResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    CoreSync::Future: Send,

    Svc: Service<InternalMessageRequest, Response = InternalMessageResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    Svc::Future: Send,

    AdrBook: Service<AddressBookRequest, Response = AddressBookResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    AdrBook::Future: Send,
{
    pub fn new(handshaker: Handshaker<Svc, CoreSync, AdrBook>) -> Self {
        Connector { handshaker }
    }
}

/// A connector request.
/// Contains the information needed to make an outbound connection to the peer.
pub struct OutboundConnectorRequest {
    /// The Monero listener address of the peer.
    pub addr: NetworkAddress,

    /// A connection tracker that reduces the open connection count when dropped.
    ///
    /// Used to limit the number of open connections in Cuprate.
    pub connection_tracker: ConnectionTracker,
}

impl<Svc, CoreSync, AdrBook> Service<OutboundConnectorRequest> for Connector<Svc, CoreSync, AdrBook>
where
    CoreSync: Service<CoreSyncDataRequest, Response = CoreSyncDataResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    CoreSync::Future: Send,

    Svc: Service<InternalMessageRequest, Response = InternalMessageResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    Svc::Future: Send,

    AdrBook: Service<AddressBookRequest, Response = AddressBookResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    AdrBook::Future: Send,
{
    type Response = (NetworkAddress, Client);
    type Error = BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: OutboundConnectorRequest) -> Self::Future {
        let OutboundConnectorRequest {
            addr: address,
            connection_tracker,
        }: OutboundConnectorRequest = req;

        let hs = self.handshaker.clone();
        let connector_span = tracing::info_span!("connector", peer = ?address);

        async move {
            let (read, write) = connect(&address).await?;
            let client = hs
                .oneshot(DoHandshakeRequest {
                    read,
                    write,
                    addr: ConnectionAddr::OutBound { address },
                    connection_tracker,
                })
                .await?;
            Ok((address, client))
        }
        .instrument(connector_span)
        .boxed()
    }
}

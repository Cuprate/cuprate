use std::{
    fmt::{Debug, Display, Formatter},
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use futures::{channel::oneshot, future::BoxFuture};
use tokio::sync::{mpsc, Semaphore};
use tower::{Service, ServiceExt};
use tracing::Instrument;

use cuprate_pruning::PruningSeed;
use cuprate_wire::{BasicNodeData, CoreSyncData};

use crate::{
    handles::{ConnectionGuard, ConnectionHandle},
    ConnectionDirection, NetworkZone, PeerError, PeerRequest, PeerResponse,
};

mod connection;
mod connector;
pub mod handshaker;
mod request_handler;
mod sync_callback;
mod timeout_monitor;

pub use connector::{ConnectRequest, Connector};
pub use handshaker::{DoHandshakeRequest, HandshakeError, HandshakerBuilder};
pub use sync_callback::PeerSyncCallback;

/// An internal identifier for a given peer, will be their address if known
/// or a random u128 if not.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum InternalPeerID<A> {
    /// A known address.
    KnownAddr(A),
    /// An unknown address (probably an inbound anonymity network connection).
    Unknown([u8; 16]),
}

impl<A: Display> Display for InternalPeerID<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KnownAddr(addr) => addr.fmt(f),
            Self::Unknown(id) => f.write_str(&format!("Unknown, ID: {}", hex::encode(id))),
        }
    }
}

/// Information on a connected peer.
#[derive(Debug, Clone)]
pub struct PeerInformation<A> {
    /// The internal peer ID of this peer.
    pub id: InternalPeerID<A>,
    /// The [`ConnectionHandle`] for this peer, allows banning this peer and checking if it is still
    /// alive.
    pub handle: ConnectionHandle,
    /// The direction of this connection (inbound|outbound).
    pub direction: ConnectionDirection,
    /// The peer's [`PruningSeed`].
    pub pruning_seed: PruningSeed,
    /// The peer's [`BasicNodeData`].
    pub basic_node_data: BasicNodeData,
    /// The [`CoreSyncData`] of this peer.
    ///
    /// Data across fields are not necessarily related, so [`CoreSyncData::top_id`] is not always the
    /// block hash for the block at height one below [`CoreSyncData::current_height`].
    ///
    /// This value is behind a [`Mutex`] and is updated whenever the peer sends new information related
    /// to their sync state. It is publicly accessible to anyone who has a peers [`Client`] handle. You
    /// probably should not mutate this value unless you are creating a custom [`ProtocolRequestHandler`](crate::ProtocolRequestHandler).
    pub core_sync_data: Arc<Mutex<CoreSyncData>>,
}

/// This represents a connection to a peer.
///
/// It allows sending requests to the peer, but does only does minimal checks that the data returned
/// is the data asked for, i.e. for a certain request the only thing checked will be that the response
/// is the correct response for that request, not that the response contains the correct data.
#[derive(Clone)]
pub struct Client<Z: NetworkZone> {
    /// Information on the connected peer.
    pub info: PeerInformation<Z::Addr>,

    /// The channel to the [`Connection`](connection::Connection) task.
    connection_tx: mpsc::Sender<connection::ConnectionTaskRequest>,
    /// The semaphore that limits the requests sent to the peer.
    semaphore: Arc<Semaphore>,
}

impl<Z: NetworkZone> Client<Z> {
    /// Creates a new [`Client`].
    pub(crate) const fn new(
        info: PeerInformation<Z::Addr>,
        connection_tx: mpsc::Sender<connection::ConnectionTaskRequest>,
        semaphore: Arc<Semaphore>,
    ) -> Self {
        Self {
            info,
            connection_tx,
            semaphore,
        }
    }

    /// Sends a request and waits for its response.
    pub async fn request(
        &self,
        request: impl Into<PeerRequest>,
    ) -> Result<PeerResponse, tower::BoxError> {
        let request = request.into();
        let (tx, rx) = oneshot::channel();
        let enqueue = async {
            let permit = if request.needs_response() {
                Some(
                    Arc::clone(&self.semaphore)
                        .acquire_owned()
                        .await
                        .map_err(|_| PeerError::ClientChannelClosed)?,
                )
            } else {
                None
            };

            self.connection_tx
                .send(connection::ConnectionTaskRequest {
                    response_channel: tx,
                    request,
                    permit,
                })
                .await
                .map_err(|_| PeerError::ClientChannelClosed)
        };

        tokio::select! {
            biased;
            () = self.info.handle.closed() => return Err(PeerError::ClientChannelClosed.into()),
            () = self.connection_tx.closed() => return Err(PeerError::ClientChannelClosed.into()),
            result = enqueue => result?,
        }

        rx.await
            .unwrap_or_else(|_| Err(PeerError::ClientChannelClosed.into()))
    }
}

impl<Z: NetworkZone, R: Into<PeerRequest>> Service<R> for Client<Z> {
    type Response = PeerResponse;
    type Error = tower::BoxError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.info.handle.is_closed() {
            return Poll::Ready(Err(PeerError::ClientChannelClosed.into()));
        }

        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: R) -> Self::Future {
        let client = self.clone();
        let request = request.into();
        Box::pin(async move { client.request(request).await })
    }
}

/// Creates a mock [`Client`] for testing purposes.
///
/// `request_handler` will be used to handle requests sent to the [`Client`]
pub fn mock_client<Z: NetworkZone, S>(
    info: PeerInformation<Z::Addr>,
    connection_guard: ConnectionGuard,
    mut request_handler: S,
) -> Client<Z>
where
    S: Service<PeerRequest, Response = PeerResponse, Error = tower::BoxError> + Send + 'static,
    S::Future: Send + 'static,
{
    let (tx, mut rx) = mpsc::channel(1);

    let task_span = tracing::error_span!("mock_connection", addr = %info.id);

    tokio::spawn(
        async move {
            let _guard = connection_guard;
            loop {
                let Some(req): Option<connection::ConnectionTaskRequest> = rx.recv().await else {
                    tracing::debug!("Channel closed, closing mock connection");
                    return;
                };

                tracing::debug!("Received new request: {:?}", req.request.id());
                let res = request_handler
                    .ready()
                    .await
                    .unwrap()
                    .call(req.request)
                    .await
                    .unwrap();

                tracing::debug!("Sending back response");

                drop(req.response_channel.send(Ok(res)));
            }
        }
        .instrument(task_span),
    );

    let semaphore = Arc::new(Semaphore::new(1));

    Client::new(info, tx, semaphore)
}

use std::{
    fmt::{Debug, Display, Formatter},
    sync::Arc,
    task::{ready, Context, Poll},
};

use futures::channel::oneshot;
use tokio::{
    sync::{mpsc, OwnedSemaphorePermit, Semaphore},
    task::JoinHandle,
};
use tokio_util::sync::PollSemaphore;
use tower::{Service, ServiceExt};
use tracing::Instrument;

use cuprate_helper::asynch::InfallibleOneshotReceiver;
use monero_pruning::PruningSeed;

use crate::{
    handles::{ConnectionGuard, ConnectionHandle},
    ConnectionDirection, NetworkZone, PeerError, PeerRequest, PeerResponse, SharedError,
};

mod connection;
mod connector;
pub mod handshaker;
mod timeout_monitor;

pub use connector::{ConnectRequest, Connector};
pub use handshaker::{DoHandshakeRequest, HandShaker, HandshakeError};

/// An internal identifier for a given peer, will be their address if known
/// or a random u128 if not.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum InternalPeerID<A> {
    /// A known address.
    KnownAddr(A),
    /// An unknown address (probably an inbound anonymity network connection).
    Unknown(u128),
}

impl<A: Display> Display for InternalPeerID<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InternalPeerID::KnownAddr(addr) => addr.fmt(f),
            InternalPeerID::Unknown(id) => f.write_str(&format!("Unknown, ID: {id}")),
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
    /// The peers pruning seed.
    pub pruning_seed: PruningSeed,
}

/// This represents a connection to a peer.
///
/// It allows sending requests to the peer, but does only does minimal checks that the data returned
/// is the data asked for, i.e. for a certain request the only thing checked will be that the response
/// is the correct response for that request, not that the response contains the correct data.
pub struct Client<Z: NetworkZone> {
    /// Information on the connected peer.
    pub info: PeerInformation<Z::Addr>,

    /// The channel to the [`Connection`](connection::Connection) task.
    connection_tx: mpsc::Sender<connection::ConnectionTaskRequest>,
    /// The [`JoinHandle`] of the spawned connection task.
    connection_handle: JoinHandle<()>,
    /// The [`JoinHandle`] of the spawned timeout monitor task.
    timeout_handle: JoinHandle<Result<(), tower::BoxError>>,

    /// The semaphore that limits the requests sent to the peer.
    semaphore: PollSemaphore,
    /// A permit for the semaphore, will be [`Some`] after `poll_ready` returns ready.
    permit: Option<OwnedSemaphorePermit>,

    /// The error slot shared between the [`Client`] and [`Connection`](connection::Connection).
    error: SharedError<PeerError>,
}

impl<Z: NetworkZone> Client<Z> {
    /// Creates a new [`Client`].
    pub(crate) fn new(
        info: PeerInformation<Z::Addr>,
        connection_tx: mpsc::Sender<connection::ConnectionTaskRequest>,
        connection_handle: JoinHandle<()>,
        timeout_handle: JoinHandle<Result<(), tower::BoxError>>,
        semaphore: Arc<Semaphore>,
        error: SharedError<PeerError>,
    ) -> Self {
        Self {
            info,
            connection_tx,
            timeout_handle,
            semaphore: PollSemaphore::new(semaphore),
            permit: None,
            connection_handle,
            error,
        }
    }

    /// Internal function to set an error on the [`SharedError`].
    fn set_err(&self, err: PeerError) -> tower::BoxError {
        let err_str = err.to_string();
        match self.error.try_insert_err(err) {
            Ok(_) => err_str,
            Err(e) => e.to_string(),
        }
        .into()
    }
}

impl<Z: NetworkZone> Service<PeerRequest> for Client<Z> {
    type Response = PeerResponse;
    type Error = tower::BoxError;
    type Future = InfallibleOneshotReceiver<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if let Some(err) = self.error.try_get_err() {
            return Poll::Ready(Err(err.to_string().into()));
        }

        if self.connection_handle.is_finished() || self.timeout_handle.is_finished() {
            let err = self.set_err(PeerError::ClientChannelClosed);
            return Poll::Ready(Err(err));
        }

        if self.permit.is_some() {
            return Poll::Ready(Ok(()));
        }

        let permit = ready!(self.semaphore.poll_acquire(cx))
            .expect("Client semaphore should not be closed!");

        self.permit = Some(permit);

        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: PeerRequest) -> Self::Future {
        let permit = self
            .permit
            .take()
            .expect("poll_ready did not return ready before call to call");

        let (tx, rx) = oneshot::channel();
        let req = connection::ConnectionTaskRequest {
            response_channel: tx,
            request,
            permit: Some(permit),
        };

        if let Err(e) = self.connection_tx.try_send(req) {
            use mpsc::error::TrySendError;

            match e {
                TrySendError::Closed(req) | TrySendError::Full(req) => {
                    self.set_err(PeerError::ClientChannelClosed);

                    let _ = req
                        .response_channel
                        .send(Err(PeerError::ClientChannelClosed.into()));
                }
            }
        }

        rx.into()
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
    S: crate::PeerRequestHandler,
{
    let (tx, mut rx) = mpsc::channel(1);

    let task_span = tracing::error_span!("mock_connection", addr = %info.id);

    let task_handle = tokio::spawn(
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

                let _ = req.response_channel.send(Ok(res));
            }
        }
        .instrument(task_span),
    );

    let timeout_task = tokio::spawn(futures::future::pending());
    let semaphore = Arc::new(Semaphore::new(1));
    let error_slot = SharedError::new();

    Client::new(info, tx, task_handle, timeout_task, semaphore, error_slot)
}

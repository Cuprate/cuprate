use std::{
    fmt::{Debug, Display, Formatter},
    sync::Arc,
    task::{ready, Context, Poll},
};

use futures::{
    channel::oneshot,
    lock::{Mutex, OwnedMutexGuard, OwnedMutexLockFuture},
    FutureExt,
};
use tokio::{
    sync::{mpsc, OwnedSemaphorePermit, Semaphore},
    task::JoinHandle,
};
use tokio_util::sync::{PollSemaphore, PollSender};
use tower::Service;

use cuprate_helper::asynch::InfallibleOneshotReceiver;
use monero_wire::{CoreSyncData, LevinCommand};

use crate::{
    handles::ConnectionHandle, ConnectionDirection, NetworkZone, PeerError, PeerRequest,
    PeerResponse, SharedError,
};

mod connection;
mod connector;
pub mod handshaker;
mod timeout_monitor;

pub use connector::{ConnectRequest, Connector};
pub use handshaker::{DoHandshakeRequest, HandShaker, HandshakeError};
use monero_pruning::PruningSeed;
use monero_wire::levin::Bucket;

/// An internal identifier for a given peer, will be their address if known
/// or a random u64 if not.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum InternalPeerID<A> {
    /// A known address
    KnownAddr(A),
    /// An unknown address (probably an inbound anonymity network connection).
    Unknown(u64),
}

impl<A: Display> Display for InternalPeerID<A> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InternalPeerID::KnownAddr(addr) => addr.fmt(f),
            InternalPeerID::Unknown(id) => f.write_str(&format!("Unknown addr, ID: {}", id)),
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

    semaphore: PollSemaphore,
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

        let Some(permit) = ready!(self.semaphore.poll_acquire(cx)) else {
            unreachable!("Client semaphore should not be closed!");
        };

        self.permit = Some(permit);

        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: PeerRequest) -> Self::Future {
        let Some(permit) = self.permit.take() else {
            panic!("poll_ready did not return ready before call to call")
        };

        let (tx, rx) = oneshot::channel();
        let req = connection::ConnectionTaskRequest {
            response_channel: tx,
            request,
            permit: Some(permit),
        };

        self.connection_tx
            .try_send(req)
            .map_err(|_| ())
            .expect("poll_ready should have been called");

        rx.into()
    }
}

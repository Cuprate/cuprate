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
use tokio::{sync::mpsc, task::JoinHandle};
use tower::Service;

use cuprate_helper::asynch::InfallibleOneshotReceiver;

use crate::{
    handles::ConnectionHandle, ConnectionDirection, NetworkZone, PeerError, PeerRequest,
    PeerResponse, SharedError,
};

mod connection;
mod connector;
pub mod handshaker;

pub use connector::{ConnectRequest, Connector};
pub use handshaker::{DoHandshakeRequest, HandShaker, HandshakeError};

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

/// This represents a connection to a peer.
///
/// It allows sending requests to the peer, but does only does minimal checks that the data returned
/// is the data asked for, i.e. for a certain request the only thing checked will be that the response
/// is the correct response for that request.
///
/// When a call to [`Client::poll_ready`] is made a slot is reserved that prevents others from sending
/// requests to this peer, so if you don't call [`Client::call`] after you must call [`Client::disarm`],
/// otherwise you prevent all requests to the peer. Even though [`Client`] does not impl [`Clone`] this
/// is still required as internal peer management code will take the mutex when it needs to send requests
/// to the peer.
pub struct Client<Z: NetworkZone> {
    /// The internal peer ID of this peer.
    id: InternalPeerID<Z::Addr>,
    /// The [`ConnectionHandle`] for this peer, allows banning this peer and checking if it is still
    /// alive.
    handle: ConnectionHandle,

    /// The direction of this connection (inbound|outbound).
    direction: ConnectionDirection,

    /// The channel to the [`Connection`](connection::Connection) task.
    connection_tx: mpsc::Sender<connection::ConnectionTaskRequest>,
    /// The [`JoinHandle`] of the spawned connection task.
    connection_handle: JoinHandle<()>,

    /// A [`Mutex`] which represents if this connection is handling a request.
    request_mutex: Arc<Mutex<()>>,
    /// A future that resolves when it is our turn to hand a request to the connection task.
    mutex_lock_fut: Option<OwnedMutexLockFuture<()>>,
    /// A guard that means we can send a request to the peer.
    mutex_lock: Option<OwnedMutexGuard<()>>,

    /// The error slot shared between the [`Client`] and [`Connection`](connection::Connection).
    error: SharedError<PeerError>,
}

impl<Z: NetworkZone> Client<Z> {
    /// Creates a new [`Client`].
    pub(crate) fn new(
        id: InternalPeerID<Z::Addr>,
        handle: ConnectionHandle,
        direction: ConnectionDirection,
        connection_tx: mpsc::Sender<connection::ConnectionTaskRequest>,
        connection_handle: JoinHandle<()>,
        request_mutex: Arc<Mutex<()>>,
        error: SharedError<PeerError>,
    ) -> Self {
        Self {
            id,
            handle,
            direction,
            connection_tx,
            connection_handle,
            request_mutex,
            mutex_lock_fut: None,
            mutex_lock: None,
            error,
        }
    }

    /// Disarms the connection, allowing other requests to be sent to the peer if we no longer need to.
    ///
    /// This *MUST* be called after a call to [`Client::poll_ready`] if you don't call [`Client::call`].
    pub fn disarm(&mut self) {
        self.mutex_lock_fut.take();
        self.mutex_lock.take();
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

        if self.connection_handle.is_finished() {
            let err = self.set_err(PeerError::ClientChannelClosed);
            return Poll::Ready(Err(err));
        }

        if self.mutex_lock.is_some() {
            return Poll::Ready(Ok(()));
        }

        loop {
            if let Some(fut) = &mut self.mutex_lock_fut {
                let _guard = ready!(fut.poll_unpin(cx));
                self.mutex_lock_fut.take();
                self.mutex_lock = Some(_guard);

                return Poll::Ready(Ok(()));
            } else {
                self.mutex_lock_fut = Some(self.request_mutex.clone().lock_owned());
            }
        }
    }

    fn call(&mut self, request: PeerRequest) -> Self::Future {
        let Some(_guard) = self.mutex_lock.take() else {
            panic!("poll_ready did not return ready");
        };

        let (tx, rx) = oneshot::channel();
        let req = connection::ConnectionTaskRequest {
            response_channel: tx,
            request,
            _guard,
        };

        self.connection_tx
            .try_send(req)
            .map_err(|_| ())
            .expect("poll_ready should have been called");

        rx.into()
    }
}

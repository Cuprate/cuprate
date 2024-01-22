use std::fmt::Formatter;
use std::{
    fmt::{Debug, Display},
    task::{Context, Poll},
};

use futures::channel::oneshot;
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_util::sync::PollSender;
use tower::Service;

use cuprate_helper::asynch::InfallibleOneshotReceiver;

use crate::{
    handles::ConnectionHandle, NetworkZone, PeerError, PeerRequest, PeerResponse, SharedError,
};

mod conector;
mod connection;
pub mod handshaker;

pub use conector::{ConnectRequest, Connector};
pub use handshaker::{DoHandshakeRequest, HandShaker, HandshakeError};

/// An internal identifier for a given peer, will be their address if known
/// or a random u64 if not.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum InternalPeerID<A> {
    KnownAddr(A),
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

pub struct Client<Z: NetworkZone> {
    id: InternalPeerID<Z::Addr>,
    handle: ConnectionHandle,

    connection_tx: PollSender<connection::ConnectionTaskRequest>,
    connection_handle: JoinHandle<()>,

    error: SharedError<PeerError>,
}

impl<Z: NetworkZone> Client<Z> {
    pub fn new(
        id: InternalPeerID<Z::Addr>,
        handle: ConnectionHandle,
        connection_tx: mpsc::Sender<connection::ConnectionTaskRequest>,
        connection_handle: JoinHandle<()>,
        error: SharedError<PeerError>,
    ) -> Self {
        Self {
            id,
            handle,
            connection_tx: PollSender::new(connection_tx),
            connection_handle,
            error,
        }
    }

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

        self.connection_tx
            .poll_reserve(cx)
            .map_err(|_| PeerError::ClientChannelClosed.into())
    }

    fn call(&mut self, request: PeerRequest) -> Self::Future {
        let (tx, rx) = oneshot::channel();
        let req = connection::ConnectionTaskRequest {
            response_channel: tx,
            request,
        };

        self.connection_tx
            .send_item(req)
            .map_err(|_| ())
            .expect("poll_ready should have been called");

        rx.into()
    }
}

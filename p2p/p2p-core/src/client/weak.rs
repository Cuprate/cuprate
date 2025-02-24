use std::task::{ready, Context, Poll};

use futures::channel::oneshot;
use tokio::sync::OwnedSemaphorePermit;
use tokio_util::sync::{PollSemaphore, PollSender};
use tower::Service;

use cuprate_helper::asynch::InfallibleOneshotReceiver;

use crate::{
    client::{connection, PeerInformation},
    BroadcastMessage, NetworkZone, PeerError, PeerRequest, PeerResponse, SharedError,
};

/// A weak handle to a [`Client`](super::Client).
///
/// When this is dropped the peer will not be disconnected.
pub struct WeakClient<N: NetworkZone> {
    /// Information on the connected peer.
    pub info: PeerInformation<N::Addr>,

    /// The channel to the [`Connection`](connection::Connection) task.
    pub(super) connection_tx: PollSender<connection::ConnectionTaskRequest>,

    /// The semaphore that limits the requests sent to the peer.
    pub(super) semaphore: PollSemaphore,
    /// A permit for the semaphore, will be [`Some`] after `poll_ready` returns ready.
    pub(super) permit: Option<OwnedSemaphorePermit>,

    /// The error slot shared between the [`Client`] and [`Connection`](connection::Connection).
    pub(super) error: SharedError<PeerError>,
}

impl<N: NetworkZone> WeakClient<N> {
    /// Internal function to set an error on the [`SharedError`].
    fn set_err(&self, err: PeerError) -> tower::BoxError {
        let err_str = err.to_string();
        match self.error.try_insert_err(err) {
            Ok(()) => err_str,
            Err(e) => e.to_string(),
        }
        .into()
    }

    pub fn broadcast_client(&mut self) -> WeakBroadcastClient<'_, N> {
        WeakBroadcastClient(self)
    }
}

impl<Z: NetworkZone> Service<PeerRequest> for WeakClient<Z> {
    type Response = PeerResponse;
    type Error = tower::BoxError;
    type Future = InfallibleOneshotReceiver<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if let Some(err) = self.error.try_get_err() {
            return Poll::Ready(Err(err.to_string().into()));
        }

        if self.permit.is_none() {
            let permit = ready!(self.semaphore.poll_acquire(cx))
                .expect("Client semaphore should not be closed!");

            self.permit = Some(permit);
        }

        if ready!(self.connection_tx.poll_reserve(cx)).is_err() {
            let err = self.set_err(PeerError::ClientChannelClosed);
            return Poll::Ready(Err(err));
        }

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

        if let Err(req) = self.connection_tx.send_item(req) {
            // The connection task could have closed between a call to `poll_ready` and the call to
            // `call`, which means if we don't handle the error here the receiver would panic.
            self.set_err(PeerError::ClientChannelClosed);

            let resp = Err(PeerError::ClientChannelClosed.into());
            drop(req.into_inner().unwrap().response_channel.send(resp));
        }

        rx.into()
    }
}

pub struct WeakBroadcastClient<'a, N: NetworkZone>(&'a mut WeakClient<N>);

impl<N: NetworkZone> Service<BroadcastMessage> for WeakBroadcastClient<'_, N> {
    type Response = PeerResponse;
    type Error = tower::BoxError;
    type Future = InfallibleOneshotReceiver<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.permit.take();

        if let Some(err) = self.0.error.try_get_err() {
            return Poll::Ready(Err(err.to_string().into()));
        }

        if ready!(self.0.connection_tx.poll_reserve(cx)).is_err() {
            let err = self.0.set_err(PeerError::ClientChannelClosed);
            return Poll::Ready(Err(err));
        }

        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: BroadcastMessage) -> Self::Future {
        let (tx, rx) = oneshot::channel();
        let req = connection::ConnectionTaskRequest {
            response_channel: tx,
            request: request.into(),
            // We don't need a permit as we only accept `BroadcastMessage`, which does not require a response.
            permit: None,
        };

        if let Err(req) = self.0.connection_tx.send_item(req) {
            // The connection task could have closed between a call to `poll_ready` and the call to
            // `call`, which means if we don't handle the error here the receiver would panic.
            self.0.set_err(PeerError::ClientChannelClosed);

            let resp = Err(PeerError::ClientChannelClosed.into());
            drop(req.into_inner().unwrap().response_channel.send(resp));
        }

        rx.into()
    }
}

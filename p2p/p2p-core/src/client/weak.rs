use std::task::{ready, Context, Poll};

use futures::channel::oneshot;
use tokio::sync::{mpsc, OwnedSemaphorePermit};
use tokio_util::sync::PollSemaphore;
use tower::Service;

use cuprate_helper::asynch::InfallibleOneshotReceiver;

use crate::{
    client::{connection, PeerInformation},
    NetworkZone, PeerError, PeerRequest, PeerResponse, SharedError,
};

pub struct WeakClient<N: NetworkZone> {
    /// Information on the connected peer.
    pub info: PeerInformation<N::Addr>,

    /// The channel to the [`Connection`](connection::Connection) task.
    pub(super) connection_tx: mpsc::WeakSender<connection::ConnectionTaskRequest>,

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
}

impl<Z: NetworkZone> Service<PeerRequest> for WeakClient<Z> {
    type Response = PeerResponse;
    type Error = tower::BoxError;
    type Future = InfallibleOneshotReceiver<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if let Some(err) = self.error.try_get_err() {
            return Poll::Ready(Err(err.to_string().into()));
        }

        if self.connection_tx.strong_count() == 0 {
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

    #[expect(clippy::significant_drop_tightening)]
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

        match self.connection_tx.upgrade() {
            None => {
                self.set_err(PeerError::ClientChannelClosed);

                let resp = Err(PeerError::ClientChannelClosed.into());
                drop(req.response_channel.send(resp));
            }
            Some(sender) => {
                if let Err(e) = sender.try_send(req) {
                    // The connection task could have closed between a call to `poll_ready` and the call to
                    // `call`, which means if we don't handle the error here the receiver would panic.
                    use mpsc::error::TrySendError;

                    match e {
                        TrySendError::Closed(req) | TrySendError::Full(req) => {
                            self.set_err(PeerError::ClientChannelClosed);

                            let resp = Err(PeerError::ClientChannelClosed.into());
                            drop(req.response_channel.send(resp));
                        }
                    }
                }
            }
        }

        rx.into()
    }
}

use std::pin::Pin;

use futures::{Future, future};
use futures::{channel::{mpsc, oneshot}, FutureExt};
use tower::Service;

use crate::protocol::{InternalMessageRequest, InternalMessageResponse};

use super::{connection::ClientRequest, PeerError};


pub struct Client {
    peer_tx: mpsc::Sender<ClientRequest>,
}

impl Service<InternalMessageRequest> for Client {
    type Error = PeerError;
    type Response = InternalMessageResponse;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        match self.peer_tx.poll_ready(cx) {
            std::task::Poll::Pending => std::task::Poll::Pending,
            std::task::Poll::Ready(res) => std::task::Poll::Ready(res.map_err(|_| PeerError::ClientChannelClosed))
        }
    }
    fn call(&mut self, req: InternalMessageRequest) -> Self::Future {
        let (tx, rx) = oneshot::channel();

        match self.peer_tx.try_send(ClientRequest {
            req,
            tx
        }) {
            Ok(()) => {
                rx.map(|recv_result| 
                    recv_result.expect("ClientRequest oneshot sender must not be dropped before send"))
                    .boxed()
            }
            Err(_e) => {
                // TODO: better error handling
                future::ready(Err(PeerError::ClientChannelClosed)).boxed()
            }
        }
    }
}
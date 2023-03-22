use std::pin::Pin;

use futures::{channel::{mpsc, oneshot}, FutureExt, Future};
use tower::Service;

use crate::protocol::{ClientReq, MessageResponse, MessageRequest};

use super::PeerError;

pub struct PeerClient {
    levin_tx: mpsc::Sender<ClientReq>
}

impl Service<MessageRequest> for PeerClient {
    type Error = PeerError;
    type Response = MessageResponse;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;
    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(())) // just for now
    }
    fn call(&mut self, req: MessageRequest) -> Self::Future {
        let (tx, rx) = oneshot::channel();
        let request = ClientReq::new(req, tx);

        match self.levin_tx.try_send(request) {
            Ok(()) => rx.boxed(),
            Err(e) => todo!("handle err")
        }
    }
}
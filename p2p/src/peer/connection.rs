use futures::channel::{mpsc, oneshot};
use futures::{Sink, SinkExt, Stream};

use monero_wire::{BucketError, Message};
use tower::{BoxError, Service, ServiceExt};

use crate::connection_handle::DisconnectSignal;
use crate::peer::error::{ErrorSlot, PeerError, SharedPeerError};
use crate::peer::handshaker::ConnectionAddr;
use crate::protocol::internal_network::{MessageID, Request, Response};

pub struct ClientRequest {
    pub req: Request,
    pub tx: oneshot::Sender<Result<Response, SharedPeerError>>,
}

pub enum State {
    WaitingForRequest,
    WaitingForResponse {
        request_id: MessageID,
        tx: oneshot::Sender<Result<Response, SharedPeerError>>,
    },
}

pub struct Connection<Svc, Snk> {
    address: ConnectionAddr,
    state: State,
    sink: Snk,
    client_rx: mpsc::Receiver<ClientRequest>,

    error_slot: ErrorSlot,

    /// # Security
    ///
    /// If this connection tracker or `Connection`s are leaked,
    /// the number of active connections will appear higher than it actually is.
    /// If enough connections leak, Cuprate will stop making new connections.
    connection_tracker: DisconnectSignal,

    svc: Svc,
}

impl<Svc, Snk> Connection<Svc, Snk>
where
    Svc: Service<Request, Response = Response, Error = BoxError>,
    Snk: Sink<Message, Error = BucketError> + Unpin,
{
    pub fn new(
        address: ConnectionAddr,
        sink: Snk,
        client_rx: mpsc::Receiver<ClientRequest>,
        error_slot: ErrorSlot,
        connection_tracker: DisconnectSignal,
        svc: Svc,
    ) -> Connection<Svc, Snk> {
        Connection {
            address,
            state: State::WaitingForRequest,
            sink,
            client_rx,
            error_slot,
            connection_tracker,
            svc,
        }
    }
    async fn handle_response(&mut self, res: Response) -> Result<(), PeerError> {
        let state = std::mem::replace(&mut self.state, State::WaitingForRequest);
        if let State::WaitingForResponse { request_id, tx } = state {
            if request_id != res.id() {
                // TODO: Fail here
                return Err(PeerError::PeerSentIncorrectResponse);
            }

            // response passed our tests we can send it to the requester
            let _ = tx.send(Ok(res));
            Ok(())
        } else {
            unreachable!("This will only be called when in state WaitingForResponse");
        }
    }

    async fn send_message_to_peer(&mut self, mes: impl Into<Message>) -> Result<(), PeerError> {
        Ok(self.sink.send(mes.into()).await?)
    }

    async fn handle_peer_request(&mut self, req: Request) -> Result<(), PeerError> {
        // we should check contents of peer requests for obvious errors like we do with responses
        todo!()
        /*
        let ready_svc = self.svc.ready().await?;
        let res = ready_svc.call(req).await?;
        self.send_message_to_peer(res).await
        */
    }

    async fn handle_client_request(&mut self, req: ClientRequest) -> Result<(), PeerError> {
        if req.req.needs_response() {
            self.state = State::WaitingForResponse {
                request_id: req.req.id(),
                tx: req.tx,
            };
        }
        // TODO: send NA response to requester
        self.send_message_to_peer(req.req).await
    }

    async fn state_waiting_for_request(&mut self) -> Result<(), PeerError> {
        futures::select! {
            peer_message = self.stream.next() => {
                match peer_message.expect("MessageStream will never return None") {
                    Ok(message) => {
                        self.handle_peer_request(message.try_into().map_err(|_| PeerError::PeerSentUnexpectedResponse)?).await
                    },
                    Err(e) => Err(e.into()),
                }
            },
            client_req = self.client_rx.next() => {
                self.handle_client_request(client_req.ok_or(PeerError::ClientChannelClosed)?).await
            },
        }
    }

    async fn state_waiting_for_response(&mut self) -> Result<(), PeerError> {
        // put a timeout on this
        let peer_message = self
            .stream
            .next()
            .await
            .expect("MessageStream will never return None")?;

        if !peer_message.is_request()
            && self.state.expected_response_id() == Some(peer_message.id())
        {
            if let Ok(res) = peer_message.try_into() {
                Ok(self.handle_response(res).await?)
            } else {
                // im almost certain this is impossible to hit, but im not certain enough to use unreachable!()
                Err(PeerError::ResponseError("Peer sent incorrect response"))
            }
        } else {
            if let Ok(req) = peer_message.try_into() {
                self.handle_peer_request(req).await
            } else {
                // this can be hit if the peer sends a protocol response with the wrong id
                Err(PeerError::ResponseError("Peer sent incorrect response"))
            }
        }
    }

    pub async fn run(mut self) {
        loop {
            let _res = match self.state {
                State::WaitingForRequest => self.state_waiting_for_request().await,
                State::WaitingForResponse { .. } => self.state_waiting_for_response().await,
            };
        }
    }
}

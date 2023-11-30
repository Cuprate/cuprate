use futures::{stream::FusedStream, SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot};

use monero_wire::Message;

use crate::{MessageID, NetworkZone, PeerError, PeerRequest, PeerRequestHandler, PeerResponse};

pub struct ConnectionTaskRequest {
    request: PeerRequest,
    response_channel: oneshot::Sender<Result<PeerResponse, PeerError>>,
}

pub enum State {
    WaitingForRequest,
    WaitingForResponse {
        request_id: MessageID,
        tx: oneshot::Sender<Result<PeerResponse, PeerError>>,
    },
}

pub struct Connection<Z: NetworkZone, ReqHndlr> {
    peer_sink: Z::Sink,

    state: State,
    client_rx: mpsc::Receiver<ConnectionTaskRequest>,

    peer_request_handler: ReqHndlr,
}

impl<Z: NetworkZone, ReqHndlr> Connection<Z, ReqHndlr>
where
    ReqHndlr: PeerRequestHandler,
{
    pub fn new(
        peer_sink: Z::Sink,
        client_rx: mpsc::Receiver<ConnectionTaskRequest>,

        peer_request_handler: ReqHndlr,
    ) -> Connection<Z, ReqHndlr> {
        Connection {
            peer_sink,
            state: State::WaitingForRequest,
            client_rx,
            peer_request_handler,
        }
    }

    async fn handle_response(&mut self, res: PeerResponse) -> Result<(), PeerError> {
        let state = std::mem::replace(&mut self.state, State::WaitingForRequest);
        if let State::WaitingForResponse { request_id, tx } = state {
            if request_id != res.id() {
                // TODO: Fail here
                return Err(PeerError::PeerSentIncorrectResponse);
            }

            // TODO: do more tests here

            // response passed our tests we can send it to the requester
            let _ = tx.send(Ok(res));
            Ok(())
        } else {
            unreachable!("This will only be called when in state WaitingForResponse");
        }
    }

    async fn send_message_to_peer(&mut self, mes: impl Into<Message>) -> Result<(), PeerError> {
        Ok(self.peer_sink.send(mes.into()).await?)
    }

    async fn handle_peer_request(&mut self, req: PeerRequest) -> Result<(), PeerError> {
        // we should check contents of peer requests for obvious errors like we do with responses
        todo!()
        /*
        let ready_svc = self.svc.ready().await?;
        let res = ready_svc.call(req).await?;
        self.send_message_to_peer(res).await
        */
    }

    async fn handle_client_request(&mut self, req: ConnectionTaskRequest) -> Result<(), PeerError> {
        if req.request.needs_response() {
            self.state = State::WaitingForResponse {
                request_id: req.request.id(),
                tx: req.response_channel,
            };
        }
        // TODO: send NA response to requester
        self.send_message_to_peer(req.request).await
    }

    async fn state_waiting_for_request<Str>(&mut self, stream: &mut Str) -> Result<(), PeerError>
    where
        Str: FusedStream<Item = Result<Message, BucketError>> + Unpin,
    {
        futures::select! {
            peer_message = stream.next() => {
                match peer_message.expect("MessageStream will never return None") {
                    Ok(message) => {
                        self.handle_peer_request(message.try_into().map_err(|_| PeerError::ResponseError(""))?).await
                    },
                    Err(e) => Err(e.into()),
                }
            },
            client_req = self.client_rx.next() => {
                self.handle_client_request(client_req.ok_or(PeerError::ClientChannelClosed)?).await
            },
        }
    }

    async fn state_waiting_for_response<Str>(&mut self, stream: &mut Str) -> Result<(), PeerError>
    where
        Str: FusedStream<Item = Result<Message, monero_wire::BucketError>> + Unpin,
    {
        // put a timeout on this
        let peer_message = stream
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

    pub async fn run<Str>(mut self, mut stream: Str)
    where
        Str: FusedStream<Item = Result<Message, monero_wire::BucketError>> + Unpin,
    {
        loop {
            let _res = match self.state {
                State::WaitingForRequest => self.state_waiting_for_request(&mut stream).await,
                State::WaitingForResponse { .. } => {
                    self.state_waiting_for_response(&mut stream).await
                }
            };
        }
    }
}

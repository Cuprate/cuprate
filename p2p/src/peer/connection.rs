
use tower::{Service, ServiceExt};
use futures::channel::{mpsc, oneshot};
use futures::{Sink, SinkExt, Stream, StreamExt};

use crate::protocol::{Request, Response};
use crate::peer::PeerError;
use monero_wire::{Message, P2pCommand};

type ClientRequestResponder = oneshot::Sender<Result<Option<Response>, PeerError>>;

pub struct ClientRequest {
    request: Request,
    tx: ClientRequestResponder
}


enum State {
    AwaitingRequest,
    AwaitingResponse{
        expected_resp: P2pCommand,
        tx: ClientRequestResponder
    }
}

pub struct Connection<Svc, Snk> {
    state: State,
    svc: Svc,
    peer_sink: Snk,
}

impl<Svc, Snk> Connection<Svc, Snk> 
where 
    Svc: Service<Request, Response = Option<Response>, Error = PeerError>,
    Snk: Sink<Message> + Unpin,
{
    async fn handle_peer_request(&mut self, req: Request) -> Result<(), PeerError> {
        match self.svc.ready().await {
            Err(e) => Err(e),
            Ok(svc) => {
                let needs_resp = req.need_response();
                if let Some(res) = svc.call(req).await? {
                    self.peer_sink.send(res.into()).await;
                } else if needs_resp {
                    return Err(PeerError::InternalServiceDidNotRespond);
                }
                Ok(())
            }

        }

    }

    async fn handle_peer_message(&mut self, message: Message) -> Result<(), PeerError> {
        let req: Request = message.try_into()?;
        self.handle_peer_request(req).await?;
        Ok(())
    }

    async fn handle_client_request(&mut self, req: ClientRequest) -> Result<(), PeerError> {
        if req.request.need_response() {
            self.state = State::AwaitingResponse { expected_resp: req.request.expected_resp(), tx: req.tx };
            self.peer_sink.send(req.request.into()).await;
        } else {
            self.peer_sink.send(req.request.into()).await;
            req.tx.send(Ok(None));
        }
        Ok(())
    }

    async fn handle_potential_peer_response(&mut self, msg: Message) -> Result<(), PeerError> {
        let prev_state = std::mem::replace(&mut self.state, State::AwaitingRequest);
        if let State::AwaitingResponse { expected_resp, tx } = prev_state {
            if expected_resp == msg.p2p_command() {
                // the P2pCommand is the same for responses and requests so if we send a request 
                // and the peer sends the same request to us the command will look like the one 
                // we are expecting
                if msg.is_request() {
                    self.state = State::AwaitingResponse { expected_resp , tx };
                    self.handle_peer_request(msg.try_into()?).await?
                } else {
                    let res: Response = msg.try_into().expect("I'm almost certain this can't be an error but should really confirm");
                    tx.send(Ok(Some(res)));
                }
                
            }
            Ok(())
        }
        else {
            unreachable!("this function will only be called when we are waiting for a response");
        }
    }

    pub async fn run<Srm : Stream<Item = Message> + Unpin>(mut self, peer_stream: Srm, internal_rx: mpsc::Receiver<ClientRequest>) {
        let mut fused_peer_stream = peer_stream.fuse();
        let mut fused_internal_requests = internal_rx.fuse();
        match self.state {
            State::AwaitingRequest => {
                futures::select! {
                    peer_message = fused_peer_stream.next() => self.handle_peer_request(peer_message.expect("Peer stream will never return None").try_into()?).await, 
                    client_req = fused_internal_requests.next() => {
                        let Some(req) = client_req else {
                            unimplemented!()
                        };
                        self.handle_client_request(req).await
                    }
                }; 
            }
            State::AwaitingResponse{..} => {
                let peer_msg = fused_peer_stream.next().await.expect("Peer stream will never return None");
                self.handle_potential_peer_response(peer_msg).await;
            }
        }
           
    }
}

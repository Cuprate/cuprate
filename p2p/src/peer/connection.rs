use futures::stream::Fuse;
use monero_wire::levin::BucketError;
use tower::{Service, ServiceExt};
use futures::channel::{mpsc, oneshot};
use futures::{Sink, SinkExt, Stream, StreamExt};

use crate::protocol::{Request, Response};
use crate::peer::PeerError;
use monero_wire::{Message, P2pCommand};

type ClientRequestResponder = oneshot::Sender<Result<Option<Response>, PeerError>>;

pub struct ClientRequest {
    request: Request,
    tx: ClientRequestResponder,
}

pub struct ConnectionTracker {
    tx: Option<oneshot::Sender<bool>>,
}

impl ConnectionTracker {
    pub fn new() -> (Self, oneshot::Receiver<bool>) {
        let (tx, rx) = oneshot::channel();
        (ConnectionTracker { tx: Some(tx) }, rx)
    }
}

impl Drop for ConnectionTracker {
    fn drop(&mut self) {
        let tx = std::mem::replace(&mut self.tx, None).expect("This function is only called once");
        tx.send(true)
            .expect("The peer count task shouldn't have finished before this one");
    }
}

enum State {
    AwaitingRequest,
    AwaitingResponse {
        expected_resp: P2pCommand,
        tx: ClientRequestResponder,
    },
}

pub struct Connection<Svc, Snk, Srm> {
    state: State,
    svc: Svc,
    peer_sink: Snk,
    peer_stream: Fuse<Srm>,
    internal_rx: Fuse<mpsc::Receiver<ClientRequest>>,
    #[allow(dead_code)]
    connection_tracker: ConnectionTracker,
}

impl<Svc, Snk, Srm> Connection<Svc, Snk, Srm>
where
    Svc: Service<Request, Response = Option<Response>, Error = PeerError>,
    Snk: Sink<Message, Error = BucketError> + Unpin,
    Srm: Stream<Item = Result<Message, BucketError>> + Unpin,
{
    pub fn new(
        peer_sink: Snk,
        peer_stream: Srm,
        svc: Svc,
        internal_rx: mpsc::Receiver<ClientRequest>,
        connection_tracker: ConnectionTracker,
    ) -> Self {
        let internal_rx = internal_rx.fuse();
        let peer_stream = peer_stream.fuse();
        Connection {
            state: State::AwaitingRequest,
            svc,
            peer_sink,
            peer_stream,
            internal_rx,
            connection_tracker,
        }
    }
    async fn handle_peer_request(&mut self, req: Request) -> Result<(), PeerError> {
        match self.svc.ready().await {
            Err(e) => Err(e),
            Ok(svc) => {
                let needs_resp = req.need_response();
                if let Some(res) = svc.call(req).await? {
                    self.peer_sink.send(res.into()).await?;
                } else if needs_resp {
                    return Err(PeerError::InternalServiceDidNotRespond);
                }
                Ok(())
            },
        }
    }

    async fn handle_client_request(&mut self, req: ClientRequest) -> Result<(), PeerError> {
        if req.request.need_response() {
            self.state = State::AwaitingResponse {
                expected_resp: req.request.expected_resp(),
                tx: req.tx,
            };
            self.peer_sink.send(req.request.into()).await?;
        } else {
            self.peer_sink.send(req.request.into()).await?;
            let _ = req.tx.send(Ok(None));
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
                    self.state = State::AwaitingResponse { expected_resp, tx };
                    self.handle_peer_request(msg.try_into()?).await?
                } else {
                    let res: Response = msg
                        .try_into()
                        .expect("I'm almost certain this can't be an error but should really confirm");
                    tx.send(Ok(Some(res))).unwrap();
                }
            }
            Ok(())
        } else {
            unreachable!("this function will only be called when we are waiting for a response");
        }
    }

    async fn state_awaiting_request(&mut self) -> Result<(), PeerError> {
        futures::select! {
            peer_message = self.peer_stream.next() => {
                let peer_message = peer_message.expect("Peer stream will never return None")?;
                self.handle_peer_request(
                    peer_message
                        .try_into()?).await
            },
            client_req = self.internal_rx.next() => {
                let Some(req) = client_req else {
                    return Err(PeerError::ClientChannelClosed);
                };
                self.handle_client_request(req).await
            }
        }
    }

    async fn state_awaiting_response(&mut self) -> Result<(), PeerError> {
        let peer_msg = self
            .peer_stream
            .next()
            .await
            .expect("Peer stream will never return None");
        self.handle_potential_peer_response(peer_msg?).await
    }

    pub async fn run(mut self) {
        loop {
            if let Err(e) = match self.state {
                State::AwaitingRequest => self.state_awaiting_request().await,
                State::AwaitingResponse { .. } => self.state_awaiting_response().await,
            } {
                self.shutdown(e).await;
                return;
            }
        }
    }

    async fn shutdown(&mut self, err: PeerError) {
        let internal_rx = self.internal_rx.get_mut();
        internal_rx.close();

        let state = std::mem::replace(&mut self.state, State::AwaitingRequest);
        if let State::AwaitingResponse { expected_resp: _, tx } = state {
            let _ = tx.send(Err(err));
        }
        while let Some(req) = internal_rx.next().await {
            let _ = req.tx.send(Err(err));
        }
    }
}

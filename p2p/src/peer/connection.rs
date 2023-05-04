use std::collections::HashSet;

use futures::channel::{mpsc, oneshot};
use futures::stream::Fuse;
use futures::{AsyncRead, AsyncWrite, SinkExt, StreamExt};

use levin::{MessageSink, MessageStream};
use monero_wire::messages::CoreSyncData;
use monero_wire::{levin, Message, NetworkAddress};
use tower::{Service, ServiceExt};

use crate::protocol::{InternalMessageRequest, InternalMessageResponse};

use super::PeerError;

pub enum PeerSyncChange {
    CoreSyncData(NetworkAddress, CoreSyncData),
    ObjectsResponse(NetworkAddress, Vec<[u8; 32]>, u64),
    PeerDisconnected(NetworkAddress),
}

pub struct ClientRequest {
    pub req: InternalMessageRequest,
    pub tx: oneshot::Sender<Result<InternalMessageResponse, PeerError>>,
}

pub enum State {
    WaitingForRequest,
    WaitingForResponse {
        request: InternalMessageRequest,
        tx: oneshot::Sender<Result<InternalMessageResponse, PeerError>>,
    },
}

impl State {
    pub fn expected_response_id(&self) -> Option<u32> {
        match self {
            Self::WaitingForRequest => None,
            Self::WaitingForResponse { request, tx: _ } => request.expected_id(),
        }
    }
}

pub struct Connection<Svc, Aw, Ar> {
    address: NetworkAddress,
    state: State,
    sink: MessageSink<Aw, Message>,
    stream: Fuse<MessageStream<Ar, Message>>,
    client_rx: mpsc::Receiver<ClientRequest>,
    sync_state_tx: mpsc::Sender<PeerSyncChange>,
    svc: Svc,
}

impl<Svc, Aw, Ar> Connection<Svc, Aw, Ar>
where
    Svc: Service<InternalMessageRequest, Response = InternalMessageResponse, Error = PeerError>,
    Aw: AsyncWrite + std::marker::Unpin,
    Ar: AsyncRead + std::marker::Unpin,
{
    pub fn new(
        address: NetworkAddress,
        sink: MessageSink<Aw, Message>,
        stream: MessageStream<Ar, Message>,
        client_rx: mpsc::Receiver<ClientRequest>,
        sync_state_tx: mpsc::Sender<PeerSyncChange>,
        svc: Svc,
    ) -> Connection<Svc, Aw, Ar> {
        Connection {
            address,
            state: State::WaitingForRequest,
            sink,
            stream: stream.fuse(),
            client_rx,
            sync_state_tx,
            svc,
        }
    }
    async fn handle_response(&mut self, res: InternalMessageResponse) -> Result<(), PeerError> {
        let state = std::mem::replace(&mut self.state, State::WaitingForRequest);
        if let State::WaitingForResponse { request, tx } = state {
            match (request, &res) {
                (InternalMessageRequest::Handshake(_), InternalMessageResponse::Handshake(_)) => {}
                (
                    InternalMessageRequest::SupportFlags(_),
                    InternalMessageResponse::SupportFlags(_),
                ) => {}
                (InternalMessageRequest::TimedSync(_), InternalMessageResponse::TimedSync(res)) => {
                }
                (
                    InternalMessageRequest::GetObjectsRequest(req),
                    InternalMessageResponse::GetObjectsResponse(res),
                ) => {}
                (
                    InternalMessageRequest::ChainRequest(_),
                    InternalMessageResponse::ChainResponse(res),
                ) => {}
                (
                    InternalMessageRequest::FluffyMissingTransactionsRequest(req),
                    InternalMessageResponse::NewFluffyBlock(blk),
                ) => {}
                (
                    InternalMessageRequest::GetTxPoolCompliment(_),
                    InternalMessageResponse::NewTransactions(_),
                ) => {
                    // we could check we received no transactions that we said we knew about but thats going to happen later anyway when they get added to our
                    // mempool
                }
                _ => return Err(PeerError::ResponseError("Peer sent incorrect response")),
            }
            // response passed our tests we can send it to the requestor
            let _ = tx.send(Ok(res));
            Ok(())
        } else {
            unreachable!("This will only be called when in state WaitingForResponse");
        }
    }

    async fn send_message_to_peer(&mut self, mes: impl Into<Message>) -> Result<(), PeerError> {
        Ok(self.sink.send(mes.into()).await?)
    }

    async fn handle_peer_request(&mut self, req: InternalMessageRequest) -> Result<(), PeerError> {
        // we should check contents of peer requests for obvious errors like we do with responses
        let ready_svc = self.svc.ready().await?;
        let res = ready_svc.call(req).await?;
        self.send_message_to_peer(res).await
    }

    async fn handle_client_request(&mut self, req: ClientRequest) -> Result<(), PeerError> {
        // check we need a response
        if let Some(_) = req.req.expected_id() {
            self.state = State::WaitingForResponse {
                request: req.req.clone(),
                tx: req.tx,
            };
        }
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
                State::WaitingForResponse { request: _, tx: _ } => {
                    self.state_waiting_for_response().await
                }
            };
        }
    }
}

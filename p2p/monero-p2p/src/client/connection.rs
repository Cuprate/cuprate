use std::sync::Arc;

use futures::{
    channel::oneshot,
    stream::{Fuse, FusedStream},
    SinkExt, StreamExt,
};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::{BroadcastStream, ReceiverStream};
use tower::ServiceExt;

use monero_wire::{LevinCommand, Message, ProtocolMessage};

use crate::{
    handles::ConnectionGuard, MessageID, NetworkZone, PeerBroadcast, PeerError, PeerRequest,
    PeerRequestHandler, PeerResponse, SharedError,
};

pub struct ConnectionTaskRequest {
    pub request: PeerRequest,
    pub response_channel: oneshot::Sender<Result<PeerResponse, tower::BoxError>>,
}

pub enum State {
    WaitingForRequest,
    WaitingForResponse {
        request_id: MessageID,
        tx: oneshot::Sender<Result<PeerResponse, tower::BoxError>>,
    },
}

/// Returns if the [`LevinCommand`] is the correct response message for our request.
///
/// e.g that we didn't get a block for a txs request.
fn levin_command_response(message_id: &MessageID, command: LevinCommand) -> bool {
    matches!(
        (message_id, command),
        (MessageID::Handshake, LevinCommand::Handshake)
            | (MessageID::TimedSync, LevinCommand::TimedSync)
            | (MessageID::Ping, LevinCommand::Ping)
            | (MessageID::SupportFlags, LevinCommand::SupportFlags)
            | (MessageID::GetObjects, LevinCommand::GetObjectsResponse)
            | (MessageID::GetChain, LevinCommand::ChainResponse)
            | (MessageID::FluffyMissingTxs, LevinCommand::NewFluffyBlock)
            | (
                MessageID::GetTxPoolCompliment,
                LevinCommand::NewTransactions
            )
    )
}

pub struct Connection<Z: NetworkZone, ReqHndlr> {
    peer_sink: Z::Sink,

    state: State,
    client_rx: Fuse<ReceiverStream<ConnectionTaskRequest>>,
    broadcast_rx: Fuse<BroadcastStream<Arc<PeerBroadcast>>>,

    peer_request_handler: ReqHndlr,

    connection_guard: ConnectionGuard,
    error: SharedError<PeerError>,
}

impl<Z: NetworkZone, ReqHndlr> Connection<Z, ReqHndlr>
where
    ReqHndlr: PeerRequestHandler,
{
    pub fn new(
        peer_sink: Z::Sink,
        client_rx: mpsc::Receiver<ConnectionTaskRequest>,
        broadcast_rx: broadcast::Receiver<Arc<PeerBroadcast>>,
        peer_request_handler: ReqHndlr,
        connection_guard: ConnectionGuard,
        error: SharedError<PeerError>,
    ) -> Connection<Z, ReqHndlr> {
        Connection {
            peer_sink,
            state: State::WaitingForRequest,
            client_rx: ReceiverStream::new(client_rx).fuse(),
            broadcast_rx: BroadcastStream::new(broadcast_rx).fuse(),
            peer_request_handler,
            connection_guard,
            error,
        }
    }

    async fn send_message_to_peer(&mut self, mes: Message) -> Result<(), PeerError> {
        Ok(self.peer_sink.send(mes).await?)
    }

    async fn handle_client_request(&mut self, req: ConnectionTaskRequest) -> Result<(), PeerError> {
        tracing::debug!("handling client request, id: {:?}", req.request.id());

        if req.request.needs_response() {
            self.state = State::WaitingForResponse {
                request_id: req.request.id(),
                tx: req.response_channel,
            };
            self.send_message_to_peer(req.request.into()).await?;
        } else {
            let res = self.send_message_to_peer(req.request.into()).await;
            if let Err(e) = res {
                let err_str = e.to_string();
                let _ = req.response_channel.send(Err(err_str.clone().into()));
                Err(e)?
            } else {
                req.response_channel.send(Ok(PeerResponse::NA));
            }
        }
        Ok(())
    }

    async fn handle_peer_request(&mut self, req: PeerRequest) -> Result<(), PeerError> {
        tracing::debug!("Received peer request: {:?}", req.id());

        let ready_svc = self.peer_request_handler.ready().await?;
        let res = ready_svc.call(req).await?;
        if matches!(res, PeerResponse::NA) {
            return Ok(());
        }

        self.send_message_to_peer(res.try_into().unwrap()).await
    }

    async fn handle_potential_response(&mut self, mes: Message) -> Result<(), PeerError> {
        tracing::debug!("Received peer message, command: {:?}", mes.command());

        if mes.is_request() {
            return self.handle_peer_request(mes.try_into().unwrap()).await;
        }

        let State::WaitingForResponse { request_id, .. } = &self.state else {
            panic!("Not in correct state, can't receive response!")
        };

        if levin_command_response(request_id, mes.command()) {
            // TODO: Do more checks before returning response.

            let State::WaitingForResponse { tx, .. } =
                std::mem::replace(&mut self.state, State::WaitingForRequest)
            else {
                panic!("Not in correct state, can't receive response!")
            };

            let _ = tx.send(Ok(mes.try_into().unwrap()));
            Ok(())
        } else {
            self.handle_peer_request(
                mes.try_into()
                    .map_err(|_| PeerError::PeerSentInvalidMessage)?,
            )
            .await
        }
    }

    async fn state_waiting_for_request<Str>(&mut self, stream: &mut Str) -> Result<(), PeerError>
    where
        Str: FusedStream<Item = Result<Message, monero_wire::BucketError>> + Unpin,
    {
        tracing::debug!("waiting for peer/client request.");
        tokio::select! {
            biased;
            broadcast_req = self.broadcast_rx.next() => {
                todo!()
            }
            client_req = self.client_rx.next() => {
                if let Some(client_req) = client_req {
                    self.handle_client_request(client_req).await
                } else {
                    Err(PeerError::ClientChannelClosed)
                }
            },
            peer_message = stream.next() => {
                if let Some(peer_message) = peer_message {
                    self.handle_peer_request(peer_message?.try_into().map_err(|_| PeerError::PeerSentInvalidMessage)?).await
                }else {
                    Err(PeerError::ClientChannelClosed)
                }
            },
        }
    }

    async fn state_waiting_for_response<Str>(&mut self, stream: &mut Str) -> Result<(), PeerError>
    where
        Str: FusedStream<Item = Result<Message, monero_wire::BucketError>> + Unpin,
    {
        tracing::debug!("waiting for peer response..");
        tokio::select! {
            biased;
            broadcast_req = self.broadcast_rx.next() => {
                todo!()
            }
            peer_message = stream.next() => {
                if let Some(peer_message) = peer_message {
                    self.handle_potential_response(peer_message?).await
                }else {
                    Err(PeerError::ClientChannelClosed)
                }
            },
        }
    }

    pub async fn run<Str>(mut self, mut stream: Str, eager_protocol_messages: Vec<ProtocolMessage>)
    where
        Str: FusedStream<Item = Result<Message, monero_wire::BucketError>> + Unpin,
    {
        tracing::debug!(
            "Handling eager messages len: {}",
            eager_protocol_messages.len()
        );
        for message in eager_protocol_messages {
            let message = Message::Protocol(message).try_into();

            let res = match message {
                Ok(mes) => self.handle_peer_request(mes).await,
                Err(_) => Err(PeerError::PeerSentInvalidMessage),
            };

            if let Err(err) = res {
                return self.shutdown(err);
            }
        }

        loop {
            if self.connection_guard.should_shutdown() {
                tracing::debug!("connection guard has shutdown, shutting down connection.");
                return self.shutdown(PeerError::ConnectionClosed);
            }

            let res = match self.state {
                State::WaitingForRequest => self.state_waiting_for_request(&mut stream).await,
                State::WaitingForResponse { .. } => {
                    self.state_waiting_for_response(&mut stream).await
                }
            };

            if let Err(err) = res {
                return self.shutdown(err);
            }
        }
    }

    fn shutdown(mut self, err: PeerError) {
        tracing::debug!("Connection task shutting down: {}", err);
        let mut client_rx = self.client_rx.into_inner().into_inner();
        client_rx.close();

        let err_str = err.to_string();
        if let Err(err) = self.error.try_insert_err(err) {
            tracing::debug!("Shared error already contains an error: {}", err);
        }

        while let Ok(req) = client_rx.try_recv() {
            let _ = req.response_channel.send(Err(err_str.clone().into()));
        }

        self.connection_guard.connection_closed();
    }
}

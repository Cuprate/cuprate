//! The Connection Task
//!
//! This module handles routing requests from a [`Client`](crate::client::Client) or a broadcast channel to
//! a peer. This module also handles routing requests from the connected peer to a request handler.
//!
use std::{pin::Pin, sync::Arc, time::Duration};

use futures::{
    channel::oneshot,
    lock::{Mutex, OwnedMutexGuard},
    stream::{Fuse, FusedStream},
    FutureExt, SinkExt, Stream, StreamExt, TryStreamExt,
};
use tokio::{
    sync::{broadcast, mpsc},
    time::{sleep, timeout, Sleep, Timeout},
};
use tokio_stream::wrappers::{errors::BroadcastStreamRecvError, BroadcastStream, ReceiverStream};
use tower::ServiceExt;

use monero_wire::{levin::Bucket, LevinCommand, Message, ProtocolMessage};

use crate::{
    constants::REQUEST_TIMEOUT, handles::ConnectionGuard, BroadcastMessage, MessageID, NetworkZone,
    PeerError, PeerRequest, PeerRequestHandler, PeerResponse, SharedError,
};

/// The timeout used when sending messages to a peer.
///
/// TODO: Make this configurable?
/// TODO: Is this a good default.
const SENDING_TIMEOUT: Duration = Duration::from_secs(20);

/// The maximum interval between messages from the peer before we send a ping message
/// to check it is still alive.
///
/// TODO: Make this configurable?
/// TODO: Is this a good default.
const TIMEOUT_INTERVAL: Duration = Duration::from_secs(60);

/// A request to the connection task from a [`Client`](crate::client::Client).
pub struct ConnectionTaskRequest {
    /// The request.
    pub request: PeerRequest,
    /// The response channel.
    pub response_channel: oneshot::Sender<Result<PeerResponse, tower::BoxError>>,
}

/// The connection state.
pub enum State {
    /// Waiting for a request from Cuprate or the connected peer.
    WaitingForRequest,
    /// Waiting for a response from the peer.
    WaitingForResponse {
        /// The requests ID.
        request_id: MessageID,
        /// The channel to send the response down.
        tx: oneshot::Sender<Result<PeerResponse, tower::BoxError>>,
    },
}

/// Returns if the [`LevinCommand`] is the correct response message for our request.
///
/// e.g. that we didn't get a block for a txs request.
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

/// This represents a connection to a peer.
pub struct Connection<Z: NetworkZone, ReqHndlr, BrdcstStrm> {
    /// The peer sink - where we send messages to the peer.
    peer_sink: Z::Sink,

    /// The connections current state.
    state: State,
    /// Will be [`Some`] if we are expecting a response from the peer.
    request_timeout: Option<Pin<Box<Sleep>>>,

    /// The client channel where requests from Cuprate to this peer will come from for us to route.
    client_rx: Fuse<ReceiverStream<ConnectionTaskRequest>>,
    broadcast_stream: Pin<Box<BrdcstStrm>>,

    /// The inner handler for any requests that come from the requested peer.
    peer_request_handler: ReqHndlr,

    /// The connection guard which will send signals to other parts of Cuprate when this connection is dropped.
    connection_guard: ConnectionGuard,
    /// An error slot which is shared with the client.
    error: SharedError<PeerError>,
}

impl<Z: NetworkZone, ReqHndlr, BrdcstStrm> Connection<Z, ReqHndlr, BrdcstStrm>
where
    ReqHndlr: PeerRequestHandler,
    BrdcstStrm: Stream<Item = BroadcastMessage> + Send + 'static,
{
    /// Create a new connection struct.
    pub fn new(
        peer_sink: Z::Sink,
        client_rx: mpsc::Receiver<ConnectionTaskRequest>,
        broadcast_stream: BrdcstStrm,
        peer_request_handler: ReqHndlr,
        connection_guard: ConnectionGuard,
        error: SharedError<PeerError>,
    ) -> Connection<Z, ReqHndlr, BrdcstStrm> {
        Connection {
            peer_sink,
            state: State::WaitingForRequest,
            request_timeout: None,
            client_rx: ReceiverStream::new(client_rx).fuse(),
            broadcast_stream: Box::pin(broadcast_stream),
            peer_request_handler,
            connection_guard,
            error,
        }
    }

    /// Sends a message to the peer, this function implements a timeout, so we don't get stuck sending a message to the
    /// peer.
    async fn send_message_to_peer(&mut self, mes: Message) -> Result<(), PeerError> {
        tracing::debug!("Sending message: [{}] to peer", mes.command());

        timeout(SENDING_TIMEOUT, self.peer_sink.send(mes.into()))
            .await
            .map_err(|_| PeerError::TimedOut)
            .and_then(|res| res.map_err(PeerError::BucketError))
    }

    /// Checks if the connection is still alive, by sending a [`PeerRequest::Ping`] and setting the state
    /// to [`State::WaitingForResponse`]. This prevents this connection from handling an internal request
    /// from Cuprate until it returns the ping response.
    async fn check_alive(&mut self) -> Result<(), PeerError> {
        // We hijack the client request handling function with a dummy request
        // to prevent duplicating code.
        let (tx, _) = oneshot::channel();

        let req = ConnectionTaskRequest {
            request: PeerRequest::Ping,
            response_channel: tx,
        };

        self.handle_client_request(req).await
    }

    async fn handle_client_broadcast(&mut self, mes: BroadcastMessage) -> Result<(), PeerError> {
        match mes {
            BroadcastMessage::NewFluffyBlock(block) => {
                self.send_message_to_peer(Message::Protocol(ProtocolMessage::NewFluffyBlock(block)))
                    .await
            }
            BroadcastMessage::NewTransaction(txs) => {
                self.send_message_to_peer(Message::Protocol(ProtocolMessage::NewTransactions(txs)))
                    .await
            }
        }
    }

    /// Handles a request from Cuprate, unlike a broadcast this request will be directed specifically at this peer.
    async fn handle_client_request(&mut self, req: ConnectionTaskRequest) -> Result<(), PeerError> {
        tracing::debug!("handling client request, id: {:?}", req.request.id());

        if req.request.needs_response() {
            self.state = State::WaitingForResponse {
                request_id: req.request.id(),
                tx: req.response_channel,
            };

            self.send_message_to_peer(req.request.into()).await?;
            // Set the timeout after sending the message, TODO: Is this a good idea.
            self.request_timeout = Some(Box::pin(sleep(REQUEST_TIMEOUT)));
        } else {
            let res = self.send_message_to_peer(req.request.into()).await;
            if let Err(e) = res {
                let err_str = e.to_string();
                let _ = req.response_channel.send(Err(err_str.clone().into()));
                Err(e)?
            } else {
                // We still need to respond even if the response is this.
                req.response_channel.send(Ok(PeerResponse::NA));
            }
        }
        Ok(())
    }

    /// Handles a request from the connected peer to this node.
    async fn handle_peer_request(&mut self, req: PeerRequest) -> Result<(), PeerError> {
        tracing::debug!("Received peer request: {:?}", req.id());

        let ready_svc = self.peer_request_handler.ready().await?;
        let res = ready_svc.call(req).await?;
        if matches!(res, PeerResponse::NA) {
            return Ok(());
        }

        self.send_message_to_peer(res.try_into().unwrap()).await
    }

    /// Handles a message from a peer when we are in [`State::WaitingForResponse`].
    async fn handle_potential_response(&mut self, mes: Message) -> Result<(), PeerError> {
        tracing::debug!("Received peer message, command: {:?}", mes.command());

        // If the message is defiantly a request then there is no way it can be a response to
        // our request.
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

            let _ = tx.send(Ok(mes
                .try_into()
                .map_err(|_| PeerError::PeerSentInvalidMessage)?));

            self.request_timeout = None;

            Ok(())
        } else {
            self.handle_peer_request(
                mes.try_into()
                    .map_err(|_| PeerError::PeerSentInvalidMessage)?,
            )
            .await
        }
    }

    /// The main-loop for when we are in [`State::WaitingForRequest`].
    async fn state_waiting_for_request<Str>(&mut self, stream: &mut Str) -> Result<(), PeerError>
    where
        Str: FusedStream<Item = Result<Message, monero_wire::BucketError>> + Unpin,
    {
        tracing::debug!("waiting for peer/client request.");

        let timeout = sleep(TIMEOUT_INTERVAL);

        tokio::select! {
            biased;
            _ = timeout => {
                self.check_alive().await
            }
            broadcast_req = self.broadcast_stream.next() => {
                if let Some(broadcast_req) = broadcast_req {
                    self.handle_client_broadcast(broadcast_req).await
                } else {
                    Err(PeerError::ClientChannelClosed)
                }
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

    /// The main-loop for when we are in [`State::WaitingForResponse`].
    async fn state_waiting_for_response<Str>(&mut self, stream: &mut Str) -> Result<(), PeerError>
    where
        Str: FusedStream<Item = Result<Message, monero_wire::BucketError>> + Unpin,
    {
        tracing::debug!("waiting for peer response.");

        tokio::select! {
            biased;
            _ = self.request_timeout.as_mut().unwrap() => {
                // TODO: Is disconnecting over the top?
                Err(PeerError::ClientChannelClosed)
            }
            broadcast_req = self.broadcast_stream.next() => {
                if let Some(broadcast_req) = broadcast_req {
                    self.handle_client_broadcast(broadcast_req).await
                } else {
                    Err(PeerError::ClientChannelClosed)
                }
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

    /// Runs the Connection handler logic, this should be put in a separate task.
    ///
    /// `eager_protocol_messages` are protocol messages that we received during a handshake.
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

    /// Shutdowns the connection, flushing pending requests and setting the error slot, if it hasn't been
    /// set already.
    fn shutdown(mut self, err: PeerError) {
        tracing::debug!("Connection task shutting down: {}", err);
        let mut client_rx = self.client_rx.into_inner().into_inner();
        client_rx.close();

        let err_str = err.to_string();
        if let Err(err) = self.error.try_insert_err(err) {
            tracing::debug!("Shared error already contains an error: {}", err);
        }

        if let State::WaitingForResponse { tx, .. } =
            std::mem::replace(&mut self.state, State::WaitingForRequest)
        {
            let _ = tx.send(Err(err_str.clone().into()));
        }

        while let Ok(req) = client_rx.try_recv() {
            let _ = req.response_channel.send(Err(err_str.clone().into()));
        }

        self.connection_guard.connection_closed();
    }
}

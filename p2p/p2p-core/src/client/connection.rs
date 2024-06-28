//! The Connection Task
//!
//! This module handles routing requests from a [`Client`](crate::client::Client) or a broadcast channel to
//! a peer. This module also handles routing requests from the connected peer to a request handler.
use std::pin::Pin;

use futures::{
    channel::oneshot,
    stream::{Fuse, FusedStream},
    SinkExt, Stream, StreamExt,
};
use tokio::{
    sync::{mpsc, OwnedSemaphorePermit},
    time::{sleep, timeout, Sleep},
};
use tokio_stream::wrappers::ReceiverStream;

use cuprate_wire::{LevinCommand, Message, ProtocolMessage};

use crate::client::request_handler::PeerRequestHandler;
use crate::{
    constants::{REQUEST_TIMEOUT, SENDING_TIMEOUT},
    handles::ConnectionGuard,
    AddressBook, BroadcastMessage, CoreSyncSvc, MessageID, NetworkZone, PeerError, PeerRequest,
    PeerResponse, PeerSyncSvc, ProtocolRequestHandler, ProtocolResponse, SharedError,
};

/// A request to the connection task from a [`Client`](crate::client::Client).
pub struct ConnectionTaskRequest {
    /// The request.
    pub request: PeerRequest,
    /// The response channel.
    pub response_channel: oneshot::Sender<Result<PeerResponse, tower::BoxError>>,
    /// A permit for this request
    pub permit: Option<OwnedSemaphorePermit>,
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
        /// A permit for this request.
        _req_permit: Option<OwnedSemaphorePermit>,
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
pub struct Connection<Z: NetworkZone, A, CS, PS, PR, BrdcstStrm> {
    /// The peer sink - where we send messages to the peer.
    peer_sink: Z::Sink,

    /// The connections current state.
    state: State,
    /// Will be [`Some`] if we are expecting a response from the peer.
    request_timeout: Option<Pin<Box<Sleep>>>,

    /// The client channel where requests from Cuprate to this peer will come from for us to route.
    client_rx: Fuse<ReceiverStream<ConnectionTaskRequest>>,
    /// A stream of messages to broadcast from Cuprate.
    broadcast_stream: Pin<Box<BrdcstStrm>>,

    /// The inner handler for any requests that come from the requested peer.
    peer_request_handler: PeerRequestHandler<Z, A, CS, PS, PR>,

    /// The connection guard which will send signals to other parts of Cuprate when this connection is dropped.
    connection_guard: ConnectionGuard,
    /// An error slot which is shared with the client.
    error: SharedError<PeerError>,
}

impl<Z, A, CS, PS, PR, BrdcstStrm> Connection<Z, A, CS, PS, PR, BrdcstStrm>
where
    Z: NetworkZone,
    A: AddressBook<Z>,
    CS: CoreSyncSvc,
    PS: PeerSyncSvc<Z>,
    PR: ProtocolRequestHandler,
    BrdcstStrm: Stream<Item = BroadcastMessage> + Send + 'static,
{
    /// Create a new connection struct.
    pub fn new(
        peer_sink: Z::Sink,
        client_rx: mpsc::Receiver<ConnectionTaskRequest>,
        broadcast_stream: BrdcstStrm,
        peer_request_handler: PeerRequestHandler<Z, A, CS, PS, PR>,
        connection_guard: ConnectionGuard,
        error: SharedError<PeerError>,
    ) -> Connection<Z, A, CS, PS, PR, BrdcstStrm> {
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

    /// Handles a broadcast request from Cuprate.
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
                _req_permit: req.permit,
            };

            self.send_message_to_peer(req.request.into()).await?;
            // Set the timeout after sending the message, TODO: Is this a good idea.
            self.request_timeout = Some(Box::pin(sleep(REQUEST_TIMEOUT)));
            return Ok(());
        }

        // INVARIANT: This function cannot exit early without sending a response back down the
        // response channel.
        let res = self.send_message_to_peer(req.request.into()).await;

        // send the response now, the request does not need a response from the peer.
        if let Err(e) = res {
            // can't clone the error so turn it to a string first, hacky but oh well.
            let err_str = e.to_string();
            let _ = req.response_channel.send(Err(err_str.clone().into()));
            return Err(e);
        } else {
            // We still need to respond even if the response is this.
            let _ = req
                .response_channel
                .send(Ok(PeerResponse::Protocol(ProtocolResponse::NA)));
        }

        Ok(())
    }

    /// Handles a request from the connected peer to this node.
    async fn handle_peer_request(&mut self, req: PeerRequest) -> Result<(), PeerError> {
        tracing::debug!("Received peer request: {:?}", req.id());

        let res = self.peer_request_handler.handle_peer_request(req).await?;

        // This will be an error if a response does not need to be sent
        if let Ok(res) = res.try_into() {
            self.send_message_to_peer(res).await?;
        }

        Ok(())
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

        // Check if the message is a response to our request.
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
        Str: FusedStream<Item = Result<Message, cuprate_wire::BucketError>> + Unpin,
    {
        tracing::debug!("waiting for peer/client request.");

        tokio::select! {
            biased;
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
        Str: FusedStream<Item = Result<Message, cuprate_wire::BucketError>> + Unpin,
    {
        tracing::debug!("waiting for peer response.");

        tokio::select! {
            biased;
            _ = self.request_timeout.as_mut().expect("Request timeout was not set!") => {
                Err(PeerError::ClientChannelClosed)
            }
            broadcast_req = self.broadcast_stream.next() => {
                if let Some(broadcast_req) = broadcast_req {
                    self.handle_client_broadcast(broadcast_req).await
                } else {
                    Err(PeerError::ClientChannelClosed)
                }
            }
            // We don't wait for client requests as we are already handling one.
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
        Str: FusedStream<Item = Result<Message, cuprate_wire::BucketError>> + Unpin,
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

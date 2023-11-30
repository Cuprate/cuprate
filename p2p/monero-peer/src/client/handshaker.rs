use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{FutureExt, SinkExt, StreamExt};
use tower::{Service, ServiceExt};
use tracing::Instrument;

use monero_wire::{
    admin::{
        HandshakeRequest, HandshakeResponse, PingResponse, SupportFlagsResponse,
        PING_OK_RESPONSE_STATUS_TEXT,
    },
    common::PeerSupportFlags,
    BasicNodeData, BucketError, CoreSyncData, Message, RequestMessage, ResponseMessage,
};

use crate::{
    AddressBook, AddressBookRequest, AddressBookResponse, ConnectionDirection, CoreSyncDataRequest,
    CoreSyncDataResponse, CoreSyncSvc, NetworkZone, PeerRequestHandler,
    MAX_PEERS_IN_PEER_LIST_MESSAGE,
};

#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    #[error("peer has the same node ID as us")]
    PeerHasSameNodeID,
    #[error("peer is on a different network")]
    IncorrectNetwork,
    #[error("peer sent a peer list with peers from different zones")]
    PeerSentIncorrectZonePeerList(#[from] crate::NetworkAddressIncorrectZone),
    #[error("peer sent invalid message: {0}")]
    PeerSentInvalidMessage(&'static str),
    #[error("Levin bucket error: {0}")]
    LevinBucketError(#[from] BucketError),
    #[error("Internal service error: {0}")]
    InternalSvcErr(#[from] tower::BoxError),
    #[error("i/o error: {0}")]
    IO(#[from] std::io::Error),
}

pub struct DoHandshakeRequest<Z: NetworkZone> {
    pub addr: Z::Addr,
    pub peer_stream: Z::Stream,
    pub peer_sink: Z::Sink,
    pub direction: ConnectionDirection,
}

#[derive(Debug, Clone)]
pub struct HandShaker<Z: NetworkZone, AdrBook, CSync, ReqHdlr> {
    address_book: AdrBook,
    core_sync_svc: CSync,
    peer_request_svc: ReqHdlr,

    our_basic_node_data: BasicNodeData,

    _zone: PhantomData<Z>,
}

impl<Z: NetworkZone, AdrBook, CSync, ReqHdlr> HandShaker<Z, AdrBook, CSync, ReqHdlr> {
    pub fn new(
        address_book: AdrBook,
        core_sync_svc: CSync,
        peer_request_svc: ReqHdlr,

        our_basic_node_data: BasicNodeData,
    ) -> Self {
        Self {
            address_book,
            core_sync_svc,
            peer_request_svc,
            our_basic_node_data,
            _zone: PhantomData,
        }
    }
}

impl<Z: NetworkZone, AdrBook, CSync, ReqHdlr> Service<DoHandshakeRequest<Z>>
    for HandShaker<Z, AdrBook, CSync, ReqHdlr>
where
    AdrBook: AddressBook<Z> + Clone,
    CSync: CoreSyncSvc + Clone,
    ReqHdlr: PeerRequestHandler + Clone,
{
    type Response = ();
    type Error = HandshakeError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: DoHandshakeRequest<Z>) -> Self::Future {
        let DoHandshakeRequest {
            addr,
            peer_stream,
            peer_sink,
            direction,
        } = req;

        let address_book = self.address_book.clone();
        let peer_request_svc = self.peer_request_svc.clone();
        let core_sync_svc = self.core_sync_svc.clone();
        let our_basic_node_data = self.our_basic_node_data.clone();

        let span = tracing::info_span!(parent: &tracing::Span::current(), "handshaker", %addr);

        let state_machine = HandshakeStateMachine::<Z, _, _, _> {
            addr,
            peer_stream,
            peer_sink,
            direction,
            address_book,
            core_sync_svc,
            peer_request_svc,
            our_basic_node_data,
            state: HandshakeState::Start,
            eager_protocol_messages: vec![],
        };

        async move {
            // TODO: timeouts
            state_machine.do_handshake().await
        }
        .instrument(span)
        .boxed()
    }
}

/// The states a handshake can be in.
#[derive(Debug, Clone, Eq, PartialEq)]
enum HandshakeState {
    /// The initial state.
    ///
    /// If this is an inbound handshake then this state means we
    /// are waiting for a [`HandshakeRequest`].
    Start,
    /// Waiting for a [`HandshakeResponse`].
    WaitingForHandshakeResponse,
    /// Waiting for a [`SupportFlagsResponse`]
    /// This contains the peers node data.
    WaitingForSupportFlagResponse(BasicNodeData, CoreSyncData),
    /// The handshake is complete.
    /// This contains the peers node data.
    Complete(BasicNodeData, CoreSyncData),
    /// An invalid state, the handshake SM should not be in this state.
    Invalid,
}

impl HandshakeState {
    /// Returns true if the handshake is completed.
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete(..))
    }

    /// returns the peers [`BasicNodeData`] and [`CoreSyncData`] if the peer
    /// is in state [`HandshakeState::Complete`].
    pub fn peer_data(self) -> Option<(BasicNodeData, CoreSyncData)> {
        match self {
            HandshakeState::Complete(bnd, coresync) => Some((bnd, coresync)),
            _ => None,
        }
    }
}

struct HandshakeStateMachine<Z: NetworkZone, AdrBook, CSync, ReqHdlr> {
    addr: Z::Addr,

    peer_stream: Z::Stream,
    peer_sink: Z::Sink,

    direction: ConnectionDirection,

    address_book: AdrBook,
    core_sync_svc: CSync,
    peer_request_svc: ReqHdlr,

    our_basic_node_data: BasicNodeData,

    state: HandshakeState,

    /// Monero allows protocol messages to be sent before a handshake response, so we have to
    /// keep track of them here. For saftey we only keep a Max of 2 messages.
    eager_protocol_messages: Vec<monero_wire::ProtocolMessage>,
}

impl<Z: NetworkZone, AdrBook, CSync, ReqHdlr> HandshakeStateMachine<Z, AdrBook, CSync, ReqHdlr>
where
    AdrBook: AddressBook<Z>,
    CSync: CoreSyncSvc,
    ReqHdlr: PeerRequestHandler,
{
    async fn send_handshake_request(&mut self) -> Result<(), HandshakeError> {
        let CoreSyncDataResponse::Ours(our_core_sync_data) = self
            .core_sync_svc
            .ready()
            .await?
            .call(CoreSyncDataRequest::Ours)
            .await?
        else {
            panic!("core sync service returned wrong response!");
        };

        let req = HandshakeRequest {
            node_data: self.our_basic_node_data.clone(),
            payload_data: our_core_sync_data,
        };

        tracing::debug!("Sending handshake request.");

        self.peer_sink
            .send(Message::Request(RequestMessage::Handshake(req)))
            .await?;

        Ok(())
    }

    async fn send_handshake_response(&mut self) -> Result<(), HandshakeError> {
        let CoreSyncDataResponse::Ours(our_core_sync_data) = self
            .core_sync_svc
            .ready()
            .await?
            .call(CoreSyncDataRequest::Ours)
            .await?
        else {
            panic!("core sync service returned wrong response!");
        };

        let AddressBookResponse::Peers(our_peer_list) = self
            .address_book
            .ready()
            .await?
            .call(AddressBookRequest::GetPeers(MAX_PEERS_IN_PEER_LIST_MESSAGE))
            .await?
        else {
            panic!("Address book sent incorrect response");
        };

        let res = HandshakeResponse {
            node_data: self.our_basic_node_data.clone(),
            payload_data: our_core_sync_data,
            local_peerlist_new: our_peer_list.into_iter().map(Into::into).collect(),
        };

        tracing::debug!("Sending handshake response.");

        self.peer_sink
            .send(Message::Response(ResponseMessage::Handshake(res)))
            .await?;

        Ok(())
    }

    async fn send_support_flags(&mut self) -> Result<(), HandshakeError> {
        let res = SupportFlagsResponse {
            support_flags: self.our_basic_node_data.support_flags,
        };

        tracing::debug!("Sending support flag response.");

        self.peer_sink
            .send(Message::Response(ResponseMessage::SupportFlags(res)))
            .await?;

        Ok(())
    }

    async fn check_request_support_flags(
        &mut self,
        support_flags: &PeerSupportFlags,
    ) -> Result<bool, HandshakeError> {
        Ok(if support_flags.is_empty() {
            tracing::debug!(
                "Peer didn't send support flags or has no features, sending request to make sure."
            );
            self.peer_sink
                .send(Message::Request(RequestMessage::SupportFlags))
                .await?;
            true
        } else {
            false
        })
    }

    async fn handle_handshake_response(
        &mut self,
        response: HandshakeResponse,
    ) -> Result<(), HandshakeError> {
        if response.local_peerlist_new.len() > MAX_PEERS_IN_PEER_LIST_MESSAGE {
            tracing::debug!("peer sent too many peers in response, cancelling handshake");

            return Err(HandshakeError::PeerSentInvalidMessage(
                "Too many peers in peer list message (>250)",
            ));
        }

        if response.node_data.network_id != self.our_basic_node_data.network_id {
            return Err(HandshakeError::IncorrectNetwork);
        }

        if Z::CHECK_NODE_ID && response.node_data.peer_id == self.our_basic_node_data.peer_id {
            return Err(HandshakeError::PeerHasSameNodeID);
        }

        tracing::debug!(
            "Telling address book about new peers, len: {}",
            response.local_peerlist_new.len()
        );

        self.address_book
            .ready()
            .await?
            .call(AddressBookRequest::IncomingPeerList(
                response
                    .local_peerlist_new
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_, _>>()?,
            ))
            .await?;

        if self
            .check_request_support_flags(&response.node_data.support_flags)
            .await?
        {
            self.state = HandshakeState::WaitingForSupportFlagResponse(
                response.node_data,
                response.payload_data,
            );
        } else {
            self.state = HandshakeState::Complete(response.node_data, response.payload_data);
        }

        Ok(())
    }

    async fn handle_handshake_request(
        &mut self,
        request: HandshakeRequest,
    ) -> Result<(), HandshakeError> {
        // We don't respond here as if we did the other peer could accept the handshake before responding to a
        // support flag request which then means we could recive other requests while waiting for the support
        // flags.

        if request.node_data.network_id != self.our_basic_node_data.network_id {
            return Err(HandshakeError::IncorrectNetwork);
        }

        if Z::CHECK_NODE_ID && request.node_data.peer_id == self.our_basic_node_data.peer_id {
            return Err(HandshakeError::PeerHasSameNodeID);
        }

        if self
            .check_request_support_flags(&request.node_data.support_flags)
            .await?
        {
            self.state = HandshakeState::WaitingForSupportFlagResponse(
                request.node_data,
                request.payload_data,
            );
        } else {
            self.state = HandshakeState::Complete(request.node_data, request.payload_data);
        }

        Ok(())
    }

    async fn handle_incoming_message(&mut self, message: Message) -> Result<(), HandshakeError> {
        tracing::debug!("Received message from peer: {}", message.command());

        if let Message::Protocol(protocol_message) = message {
            if self.eager_protocol_messages.len() == 2 {
                tracing::debug!("Peer sent too many protocl messages before a handshake response.");
                return Err(HandshakeError::PeerSentInvalidMessage(
                    "Peer sent too many protocol messages",
                ));
            }
            tracing::debug!(
                "Protocol message getting added to queue for when handshake is complete."
            );
            self.eager_protocol_messages.push(protocol_message);
            return Ok(());
        }

        match std::mem::replace(&mut self.state, HandshakeState::Invalid) {
            HandshakeState::Start => match message {
                Message::Request(RequestMessage::Ping) => {
                    // Set the state back to what it was before.
                    self.state = HandshakeState::Start;
                    Ok(self
                        .peer_sink
                        .send(Message::Response(ResponseMessage::Ping(PingResponse {
                            status: PING_OK_RESPONSE_STATUS_TEXT.to_string(),
                            peer_id: self.our_basic_node_data.peer_id,
                        })))
                        .await?)
                }
                Message::Request(RequestMessage::Handshake(handshake_req)) => {
                    self.handle_handshake_request(handshake_req).await
                }
                _ => Err(HandshakeError::PeerSentInvalidMessage(
                    "Peer didn't send handshake request.",
                )),
            },
            HandshakeState::WaitingForHandshakeResponse => match message {
                // TODO: only allow 1 support flag request.
                Message::Request(RequestMessage::SupportFlags) => {
                    // Set the state back to what it was before.
                    self.state = HandshakeState::WaitingForHandshakeResponse;
                    self.send_support_flags().await
                }
                Message::Response(ResponseMessage::Handshake(res)) => {
                    self.handle_handshake_response(res).await
                }
                _ => Err(HandshakeError::PeerSentInvalidMessage(
                    "Peer didn't send handshake response.",
                )),
            },
            HandshakeState::WaitingForSupportFlagResponse(mut peer_node_data, peer_core_sync) => {
                let Message::Response(ResponseMessage::SupportFlags(support_flags)) = message
                else {
                    return Err(HandshakeError::PeerSentInvalidMessage(
                        "Peer didn't send support flags response.",
                    ));
                };
                peer_node_data.support_flags = support_flags.support_flags;
                self.state = HandshakeState::Complete(peer_node_data, peer_core_sync);
                Ok(())
            }
            HandshakeState::Complete(..) => {
                panic!("Handshake is complete messages should no longer be handled here!")
            }
            HandshakeState::Invalid => panic!("Handshake state machine stayed in invalid state!"),
        }
    }

    async fn advance_machine(&mut self) -> Result<(), HandshakeError> {
        while !self.state.is_complete() {
            tracing::debug!("Waiting for message from peer.");

            match self.peer_stream.next().await {
                Some(message) => self.handle_incoming_message(message?).await?,
                None => Err(BucketError::IO(std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "The peer stream returned None",
                )))?,
            }
        }

        Ok(())
    }

    async fn do_outbound_handshake(&mut self) -> Result<(), HandshakeError> {
        self.send_handshake_request().await?;
        self.state = HandshakeState::WaitingForHandshakeResponse;

        self.advance_machine().await
    }

    async fn do_inbound_handshake(&mut self) -> Result<(), HandshakeError> {
        self.advance_machine().await?;

        debug_assert!(self.state.is_complete());

        self.send_handshake_response().await
    }

    async fn do_handshake(mut self) -> Result<(), HandshakeError> {
        tracing::debug!("Beginning handshake.");

        match self.direction {
            ConnectionDirection::OutBound => self.do_outbound_handshake().await?,
            ConnectionDirection::InBound => self.do_inbound_handshake().await?,
        }

        let HandshakeState::Complete(peer_node_data, peer_core_sync) = self.state else {
            panic!("Hanshake completed not in complete state!");
        };

        self.core_sync_svc
            .ready()
            .await?
            .call(CoreSyncDataRequest::HandleIncoming(peer_core_sync))
            .await?;

        tracing::debug!("Handshake complete.");

        Ok(())
    }
}

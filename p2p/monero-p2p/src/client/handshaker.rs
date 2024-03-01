//! The Handshaker
//!
//! This module handles handshaking with a peer, be it an outbound or inbound handshake and returns a [`Client`]
//! which requests can be sent to.
//!
//! If you are looking to do outbound handshakes the [`Connector`](super::Connector) wraps this and is what you are looking for.
//!
use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use futures::{lock::Mutex, FutureExt, SinkExt, StreamExt};
use tokio::{
    sync::{broadcast, mpsc, OwnedSemaphorePermit},
    time::{error::Elapsed, timeout},
};
use tower::{Service, ServiceExt};
use tracing::Instrument;

use monero_wire::{
    admin::{
        HandshakeRequest, HandshakeResponse, PingResponse, SupportFlagsResponse,
        PING_OK_RESPONSE_STATUS_TEXT,
    },
    common::PeerSupportFlags,
    BasicNodeData, BucketError, CoreSyncData, LevinCommand, Message, ProtocolMessage,
    RequestMessage, ResponseMessage,
};

use crate::{
    client::{connection::Connection, Client, InternalPeerID, PeerInformation},
    handles::HandleBuilder,
    AddressBook, AddressBookRequest, AddressBookResponse, ConnectionDirection, CoreSyncDataRequest,
    CoreSyncDataResponse, CoreSyncSvc, MessageID, NetworkZone, PeerBroadcast, PeerRequestHandler,
    SharedError, MAX_PEERS_IN_PEER_LIST_MESSAGE,
};

/// This is Cuprate specific - monerod will send protocol messages before a handshake is complete in
/// certain circumstances i.e. monerod will send a [`ProtocolMessage::GetTxPoolCompliment`] if our node
/// is its first connection, and we are at the same height.
///
/// Cuprate needs to complete a handshake before any protocol messages can be handled though, so we keep
/// them around to handle when the handshake is done. We don't want this to grow forever though, so we cap
/// the amount we can receive.
const MAX_EAGER_PROTOCOL_MESSAGES: usize = 2;

/// A timeout for a handshake - the handshake must complete before this.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(120);

/// An error that can be returned from a handshake.
#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    /// The handshake timed-out.
    #[error("The handshake timed out")]
    TimedOut(#[from] Elapsed),
    /// The peer has the same node ID as us and we are on a NetworkZone where
    /// node IDs are checked.
    #[error("Peer has the same node ID as us")]
    PeerHasSameNodeID,
    /// The peer is on a different network to us.
    #[error("Peer is on a different network")]
    IncorrectNetwork,
    /// The peer does not support a feature we require.
    #[error("The peer does not meet one or more of the required features")]
    PeerDoesNotHaveRequiredFeature,
    /// The peer sent a peer list of nodes which contain peers on a different
    /// network zone.
    #[error("Peer sent a peer list with peers from different zones")]
    PeerSentIncorrectPeerList(#[from] crate::services::PeerListConversionError),
    /// The peer sent an invalid message.
    #[error("Peer sent invalid message: {0}")]
    PeerSentInvalidMessage(&'static str),
    /// A levin error.
    #[error("Levin bucket error: {0}")]
    LevinBucketError(#[from] BucketError),
    /// An error from an internal service.
    #[error("Internal service error: {0}")]
    InternalSvcErr(#[from] tower::BoxError),
    /// I/O error.
    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),
}

/// A request to complete a handshake with a peer.
pub struct DoHandshakeRequest<Z: NetworkZone> {
    /// The internal peer ID of this peer.
    pub peer_id: InternalPeerID<Z::Addr>,
    /// The message stream from this peer.
    pub peer_stream: Z::Stream,
    /// The message sink to this peer.
    pub peer_sink: Z::Sink,
    /// The direction of this peer connection.
    pub direction: ConnectionDirection,
    /// A permit that the connection task will hold for the duration of the connection.
    pub permit: OwnedSemaphorePermit,
}

/// The handshaker service which performs a handshake with a peer returning a [`Client`].
#[derive(Debug, Clone)]
pub struct HandShaker<Z: NetworkZone, AdrBook, CSync, ReqHdlr> {
    /// The address book service.
    address_book: AdrBook,
    /// The core sync service.
    core_sync_svc: CSync,
    /// The peer request handler service.
    peer_request_svc: ReqHdlr,

    /// The support flags that must be set by the peer for a handshake to succeed.
    minimum_support_flags: PeerSupportFlags,

    /// Our basic node data, for this network.
    our_basic_node_data: BasicNodeData,

    /// The broadcast channel that messages that need to be sent to every peer will be sent down.
    /// Although inbound and outbound peers will have different channels
    broadcast_tx: broadcast::Sender<PeerBroadcast>,

    /// The network zone for this handshaker.
    _zone: PhantomData<Z>,
}

impl<Z: NetworkZone, AdrBook, CSync, ReqHdlr> HandShaker<Z, AdrBook, CSync, ReqHdlr> {
    /// Create a new [`HandShaker`]
    pub fn new(
        address_book: AdrBook,
        core_sync_svc: CSync,
        peer_request_svc: ReqHdlr,

        minimum_support_flags: Option<PeerSupportFlags>,

        broadcast_tx: broadcast::Sender<PeerBroadcast>,

        our_basic_node_data: BasicNodeData,
    ) -> Self {
        Self {
            address_book,
            core_sync_svc,
            peer_request_svc,
            minimum_support_flags: minimum_support_flags.unwrap_or(PeerSupportFlags::empty()),
            broadcast_tx,
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
    type Response = Client<Z>;
    type Error = HandshakeError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: DoHandshakeRequest<Z>) -> Self::Future {
        let broadcast_rx = self.broadcast_tx.subscribe();

        let minimum_support_flags = self.minimum_support_flags;

        let address_book = self.address_book.clone();
        let peer_request_svc = self.peer_request_svc.clone();
        let core_sync_svc = self.core_sync_svc.clone();
        let our_basic_node_data = self.our_basic_node_data.clone();

        let span =
            tracing::info_span!(parent: &tracing::Span::current(), "handshaker", %req.peer_id);

        async move {
            timeout(
                HANDSHAKE_TIMEOUT,
                handshake(
                    req,
                    minimum_support_flags,
                    broadcast_rx,
                    address_book,
                    core_sync_svc,
                    peer_request_svc,
                    our_basic_node_data,
                ),
            )
            .await?
        }
        .instrument(span)
        .boxed()
    }
}

/// This function completes a handshake with the requested peer.
async fn handshake<Z: NetworkZone, AdrBook, CSync, ReqHdlr>(
    req: DoHandshakeRequest<Z>,

    minimum_support_flags: PeerSupportFlags,

    broadcast_rx: broadcast::Receiver<PeerBroadcast>,

    mut address_book: AdrBook,
    mut core_sync_svc: CSync,
    peer_request_svc: ReqHdlr,
    our_basic_node_data: BasicNodeData,
) -> Result<Client<Z>, HandshakeError>
where
    AdrBook: AddressBook<Z>,
    CSync: CoreSyncSvc,
    ReqHdlr: PeerRequestHandler,
{
    let DoHandshakeRequest {
        peer_id: addr,
        mut peer_stream,
        mut peer_sink,
        direction,
        permit,
    } = req;

    // See [`MAX_EAGER_PROTOCOL_MESSAGES`]
    let mut eager_protocol_messages = Vec::new();
    let mut allow_support_flag_req = true;

    let (peer_core_sync, mut peer_node_data) = match direction {
        ConnectionDirection::InBound => {
            tracing::debug!("waiting for handshake request.");

            let Message::Request(RequestMessage::Handshake(handshake_req)) = wait_for_message::<Z>(
                LevinCommand::Handshake,
                true,
                &mut peer_sink,
                &mut peer_stream,
                &mut eager_protocol_messages,
                &mut allow_support_flag_req,
                our_basic_node_data.support_flags,
            )
            .await?
            else {
                panic!("wait_for_message returned ok with wrong message.");
            };

            tracing::debug!("Received handshake request.");

            (handshake_req.payload_data, handshake_req.node_data)
        }
        ConnectionDirection::OutBound => {
            send_hs_request::<Z, _>(
                &mut peer_sink,
                &mut core_sync_svc,
                our_basic_node_data.clone(),
            )
            .await?;

            let Message::Response(ResponseMessage::Handshake(handshake_res)) =
                wait_for_message::<Z>(
                    LevinCommand::Handshake,
                    false,
                    &mut peer_sink,
                    &mut peer_stream,
                    &mut eager_protocol_messages,
                    &mut allow_support_flag_req,
                    our_basic_node_data.support_flags,
                )
                .await?
            else {
                panic!("wait_for_message returned ok with wrong message.");
            };

            if handshake_res.local_peerlist_new.len() > MAX_PEERS_IN_PEER_LIST_MESSAGE {
                tracing::debug!("peer sent too many peers in response, cancelling handshake");

                return Err(HandshakeError::PeerSentInvalidMessage(
                    "Too many peers in peer list message (>250)",
                ));
            }

            tracing::debug!(
                "Telling address book about new peers, len: {}",
                handshake_res.local_peerlist_new.len()
            );

            address_book
                .ready()
                .await?
                .call(AddressBookRequest::IncomingPeerList(
                    handshake_res
                        .local_peerlist_new
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<_, _>>()?,
                ))
                .await?;

            (handshake_res.payload_data, handshake_res.node_data)
        }
    };

    if peer_node_data.network_id != our_basic_node_data.network_id {
        return Err(HandshakeError::IncorrectNetwork);
    }

    if Z::CHECK_NODE_ID && peer_node_data.peer_id == our_basic_node_data.peer_id {
        return Err(HandshakeError::PeerHasSameNodeID);
    }

    if peer_node_data.support_flags.is_empty() {
        tracing::debug!(
            "Peer didn't send support flags or has no features, sending request to make sure."
        );
        peer_sink
            .send(Message::Request(RequestMessage::SupportFlags))
            .await?;

        let Message::Response(ResponseMessage::SupportFlags(support_flags_res)) =
            wait_for_message::<Z>(
                LevinCommand::SupportFlags,
                false,
                &mut peer_sink,
                &mut peer_stream,
                &mut eager_protocol_messages,
                &mut allow_support_flag_req,
                our_basic_node_data.support_flags,
            )
            .await?
        else {
            panic!("wait_for_message returned ok with wrong message.");
        };

        tracing::debug!("Received support flag response.");
        peer_node_data.support_flags = support_flags_res.support_flags;
    }

    if !peer_node_data.support_flags.contains(minimum_support_flags) {
        tracing::debug!("Peer does not meet the minimum supported features, dropping connection.");
        return Err(HandshakeError::PeerDoesNotHaveRequiredFeature);
    }

    if direction == ConnectionDirection::InBound {
        send_hs_response::<Z, _, _>(
            &mut peer_sink,
            &mut core_sync_svc,
            &mut address_book,
            our_basic_node_data,
        )
        .await?;
    }

    core_sync_svc
        .ready()
        .await?
        .call(CoreSyncDataRequest::HandleIncoming(peer_core_sync.clone()))
        .await?;

    tracing::debug!("Handshake complete.");

    let error_slot = SharedError::new();

    let (connection_guard, handle, _) = HandleBuilder::new().with_permit(permit).build();

    let (connection_tx, client_rx) = mpsc::channel(1);

    let connection = Connection::<Z, _>::new(
        peer_sink,
        client_rx,
        broadcast_rx,
        peer_request_svc,
        connection_guard,
        error_slot.clone(),
    );

    let connection_handle =
        tokio::spawn(connection.run(peer_stream.fuse(), eager_protocol_messages));

    let peer_info = Arc::new(PeerInformation {
        id: addr,
        handle,
        direction,
        core_sync_data: std::sync::Mutex::new(peer_core_sync),
    });

    let client = Client::<Z>::new(peer_info, connection_tx, connection_handle, error_slot);

    Ok(client)
}

/// Sends a [`HandshakeRequest`] to the peer.
async fn send_hs_request<Z: NetworkZone, CSync>(
    peer_sink: &mut Z::Sink,
    core_sync_svc: &mut CSync,
    our_basic_node_data: BasicNodeData,
) -> Result<(), HandshakeError>
where
    CSync: CoreSyncSvc,
{
    let CoreSyncDataResponse::Ours(our_core_sync_data) = core_sync_svc
        .ready()
        .await?
        .call(CoreSyncDataRequest::Ours)
        .await?
    else {
        panic!("core sync service returned wrong response!");
    };

    let req = HandshakeRequest {
        node_data: our_basic_node_data,
        payload_data: our_core_sync_data,
    };

    tracing::debug!("Sending handshake request.");

    peer_sink
        .send(Message::Request(RequestMessage::Handshake(req)))
        .await?;

    Ok(())
}

/// Sends a [`HandshakeResponse`] to the peer.
async fn send_hs_response<Z: NetworkZone, CSync, AdrBook>(
    peer_sink: &mut Z::Sink,
    core_sync_svc: &mut CSync,
    address_book: &mut AdrBook,
    our_basic_node_data: BasicNodeData,
) -> Result<(), HandshakeError>
where
    AdrBook: AddressBook<Z>,
    CSync: CoreSyncSvc,
{
    let CoreSyncDataResponse::Ours(our_core_sync_data) = core_sync_svc
        .ready()
        .await?
        .call(CoreSyncDataRequest::Ours)
        .await?
    else {
        panic!("core sync service returned wrong response!");
    };

    let AddressBookResponse::Peers(our_peer_list) = address_book
        .ready()
        .await?
        .call(AddressBookRequest::GetWhitePeers(
            MAX_PEERS_IN_PEER_LIST_MESSAGE,
        ))
        .await?
    else {
        panic!("Address book sent incorrect response");
    };

    let res = HandshakeResponse {
        node_data: our_basic_node_data,
        payload_data: our_core_sync_data,
        local_peerlist_new: our_peer_list.into_iter().map(Into::into).collect(),
    };

    tracing::debug!("Sending handshake response.");

    peer_sink
        .send(Message::Response(ResponseMessage::Handshake(res)))
        .await?;

    Ok(())
}

/// This function waits for a specific P2P message, handling other messages, if they are valid.
async fn wait_for_message<Z: NetworkZone>(
    levin_command: LevinCommand,
    request: bool,
    peer_sink: &mut Z::Sink,
    peer_stream: &mut Z::Stream,
    eager_protocol_messages: &mut Vec<ProtocolMessage>,
    allow_support_flag_req: &mut bool,
    support_flags: PeerSupportFlags,
) -> Result<Message, HandshakeError> {
    while let Some(message) = peer_stream.next().await {
        let message = message?;

        match message {
            Message::Protocol(protocol_message) => {
                // No protocol messages are exchanged during a normal handshake.
                tracing::debug!(
                    "Received eager protocol message with ID: {}, adding to queue",
                    protocol_message.command()
                );
                eager_protocol_messages.push(protocol_message);
                if eager_protocol_messages.len() > MAX_EAGER_PROTOCOL_MESSAGES {
                    tracing::debug!(
                        "Peer sent too many protocol messages before a handshake response."
                    );
                    return Err(HandshakeError::PeerSentInvalidMessage(
                        "Peer sent too many protocol messages",
                    ));
                }
                continue;
            }
            Message::Request(req_message) => {
                if req_message.command() == levin_command && request {
                    return Ok(Message::Request(req_message));
                }

                // The only valid request while waiting for a message is a SupportFlags request
                if matches!(req_message, RequestMessage::SupportFlags) {
                    if !*allow_support_flag_req {
                        return Err(HandshakeError::PeerSentInvalidMessage(
                            "Peer sent 2 support flag requests",
                        ));
                    }
                    send_support_flags::<Z>(peer_sink, support_flags).await?;
                    // don't let the peer send more after the first request.
                    *allow_support_flag_req = false;
                    continue;
                }

                return Err(HandshakeError::PeerSentInvalidMessage(
                    "Peer sent an admin request before responding to the handshake",
                ));
            }
            Message::Response(res_message) if !request => {
                if res_message.command() == levin_command {
                    return Ok(Message::Response(res_message));
                }

                // The peer can only respond to our request it can't send a random response.
                tracing::debug!("Received unexpected response: {}", res_message.command());

                return Err(HandshakeError::PeerSentInvalidMessage(
                    "Peer sent an incorrect response",
                ));
            }

            _ => Err(HandshakeError::PeerSentInvalidMessage(
                "Peer sent an incorrect message",
            )),
        }?
    }

    Err(BucketError::IO(std::io::Error::new(
        std::io::ErrorKind::ConnectionAborted,
        "The peer stream returned None",
    )))?
}

/// Sends our support flags to the peer.
async fn send_support_flags<Z: NetworkZone>(
    peer_sink: &mut Z::Sink,
    support_flags: PeerSupportFlags,
) -> Result<(), HandshakeError> {
    tracing::debug!("Sending support flag response.");
    Ok(peer_sink
        .send(Message::Response(ResponseMessage::SupportFlags(
            SupportFlagsResponse { support_flags },
        )))
        .await?)
}

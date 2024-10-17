//! Handshake Module
//!
//! This module contains a [`HandShaker`] which is a [`Service`] that takes an open connection and attempts
//! to complete a handshake with them.
//!
//! This module also contains a [`ping`] function that can be used to check if an address is reachable.
use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use futures::{FutureExt, SinkExt, Stream, StreamExt};
use tokio::{
    sync::{mpsc, OwnedSemaphorePermit, Semaphore},
    time::{error::Elapsed, timeout},
};
use tower::{Service, ServiceExt};
use tracing::{info_span, Instrument, Span};

use cuprate_pruning::{PruningError, PruningSeed};
use cuprate_wire::{
    admin::{
        HandshakeRequest, HandshakeResponse, PingResponse, SupportFlagsResponse,
        PING_OK_RESPONSE_STATUS_TEXT,
    },
    common::PeerSupportFlags,
    AdminRequestMessage, AdminResponseMessage, BasicNodeData, BucketError, LevinCommand, Message,
};

use crate::{
    client::{
        connection::Connection, request_handler::PeerRequestHandler,
        timeout_monitor::connection_timeout_monitor_task, Client, InternalPeerID, PeerInformation,
    },
    constants::{
        HANDSHAKE_TIMEOUT, MAX_EAGER_PROTOCOL_MESSAGES, MAX_PEERS_IN_PEER_LIST_MESSAGE,
        PING_TIMEOUT,
    },
    handles::HandleBuilder,
    AddressBook, AddressBookRequest, AddressBookResponse, BroadcastMessage, ConnectionDirection,
    CoreSyncDataRequest, CoreSyncDataResponse, CoreSyncSvc, NetZoneAddress, NetworkZone,
    ProtocolRequestHandlerMaker, SharedError,
};

pub mod builder;
pub use builder::HandshakerBuilder;

#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    #[error("The handshake timed out")]
    TimedOut(#[from] Elapsed),
    #[error("Peer has the same node ID as us")]
    PeerHasSameNodeID,
    #[error("Peer is on a different network")]
    IncorrectNetwork,
    #[error("Peer sent a peer list with peers from different zones")]
    PeerSentIncorrectPeerList(#[from] crate::services::PeerListConversionError),
    #[error("Peer sent invalid message: {0}")]
    PeerSentInvalidMessage(&'static str),
    #[error("The peers pruning seed is invalid.")]
    InvalidPruningSeed(#[from] PruningError),
    #[error("Levin bucket error: {0}")]
    LevinBucketError(#[from] BucketError),
    #[error("Internal service error: {0}")]
    InternalSvcErr(#[from] tower::BoxError),
    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),
}

/// A request to complete a handshake.
pub struct DoHandshakeRequest<Z: NetworkZone> {
    /// The [`InternalPeerID`] of the peer we are handshaking with.
    pub addr: InternalPeerID<Z::Addr>,
    /// The receiving side of the connection.
    pub peer_stream: Z::Stream,
    /// The sending side of the connection.
    pub peer_sink: Z::Sink,
    /// The direction of the connection.
    pub direction: ConnectionDirection,
    /// An [`Option`]al permit for this connection.
    pub permit: Option<OwnedSemaphorePermit>,
}

/// The peer handshaking service.
#[derive(Debug, Clone)]
pub struct HandShaker<Z: NetworkZone, AdrBook, CSync, ProtoHdlrMkr, BrdcstStrmMkr> {
    /// The address book service.
    address_book: AdrBook,
    /// The core sync data service.
    core_sync_svc: CSync,
    /// The protocol request handler service.
    protocol_request_svc_maker: ProtoHdlrMkr,

    /// Our [`BasicNodeData`]
    our_basic_node_data: BasicNodeData,

    /// A function that returns a stream that will give items to be broadcast by a connection.
    broadcast_stream_maker: BrdcstStrmMkr,

    connection_parent_span: Span,

    /// The network zone.
    _zone: PhantomData<Z>,
}

impl<Z: NetworkZone, AdrBook, CSync, ProtoHdlrMkr, BrdcstStrmMkr>
    HandShaker<Z, AdrBook, CSync, ProtoHdlrMkr, BrdcstStrmMkr>
{
    /// Creates a new handshaker.
    const fn new(
        address_book: AdrBook,
        core_sync_svc: CSync,
        protocol_request_svc_maker: ProtoHdlrMkr,
        broadcast_stream_maker: BrdcstStrmMkr,
        our_basic_node_data: BasicNodeData,
        connection_parent_span: Span,
    ) -> Self {
        Self {
            address_book,
            core_sync_svc,
            protocol_request_svc_maker,
            broadcast_stream_maker,
            our_basic_node_data,
            connection_parent_span,
            _zone: PhantomData,
        }
    }
}

impl<Z: NetworkZone, AdrBook, CSync, ProtoHdlrMkr, BrdcstStrmMkr, BrdcstStrm>
    Service<DoHandshakeRequest<Z>> for HandShaker<Z, AdrBook, CSync, ProtoHdlrMkr, BrdcstStrmMkr>
where
    AdrBook: AddressBook<Z> + Clone,
    CSync: CoreSyncSvc + Clone,
    ProtoHdlrMkr: ProtocolRequestHandlerMaker<Z> + Clone,
    BrdcstStrm: Stream<Item = BroadcastMessage> + Send + 'static,
    BrdcstStrmMkr: Fn(InternalPeerID<Z::Addr>) -> BrdcstStrm + Clone + Send + 'static,
{
    type Response = Client<Z>;
    type Error = HandshakeError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: DoHandshakeRequest<Z>) -> Self::Future {
        let broadcast_stream_maker = self.broadcast_stream_maker.clone();

        let address_book = self.address_book.clone();
        let protocol_request_svc_maker = self.protocol_request_svc_maker.clone();
        let core_sync_svc = self.core_sync_svc.clone();
        let our_basic_node_data = self.our_basic_node_data.clone();

        let connection_parent_span = self.connection_parent_span.clone();

        let span = info_span!(parent: &Span::current(), "handshaker", addr=%req.addr);

        async move {
            timeout(
                HANDSHAKE_TIMEOUT,
                handshake(
                    req,
                    broadcast_stream_maker,
                    address_book,
                    core_sync_svc,
                    protocol_request_svc_maker,
                    our_basic_node_data,
                    connection_parent_span,
                ),
            )
            .await?
        }
        .instrument(span)
        .boxed()
    }
}

/// Send a ping to the requested peer and wait for a response, returning the `peer_id`.
///
/// This function does not put a timeout on the ping.
pub async fn ping<N: NetworkZone>(addr: N::Addr) -> Result<u64, HandshakeError> {
    tracing::debug!("Sending Ping to peer");

    let (mut peer_stream, mut peer_sink) = N::connect_to_peer(addr).await?;

    tracing::debug!("Made outbound connection to peer, sending ping.");

    peer_sink
        .send(Message::Request(AdminRequestMessage::Ping).into())
        .await?;

    if let Some(res) = peer_stream.next().await {
        if let Message::Response(AdminResponseMessage::Ping(ping)) = res? {
            if ping.status == PING_OK_RESPONSE_STATUS_TEXT {
                tracing::debug!("Ping successful.");
                return Ok(ping.peer_id);
            }

            tracing::debug!("Peer's ping response was not `OK`.");
            return Err(HandshakeError::PeerSentInvalidMessage(
                "Ping response was not `OK`",
            ));
        }

        tracing::debug!("Peer sent invalid response to ping.");
        return Err(HandshakeError::PeerSentInvalidMessage(
            "Peer did not send correct response for ping.",
        ));
    }

    tracing::debug!("Connection closed before ping response.");
    Err(BucketError::IO(std::io::Error::new(
        std::io::ErrorKind::ConnectionAborted,
        "The peer stream returned None",
    ))
    .into())
}

/// This function completes a handshake with the requested peer.
async fn handshake<Z: NetworkZone, AdrBook, CSync, ProtoHdlrMkr, BrdcstStrmMkr, BrdcstStrm>(
    req: DoHandshakeRequest<Z>,

    broadcast_stream_maker: BrdcstStrmMkr,

    mut address_book: AdrBook,
    mut core_sync_svc: CSync,
    mut protocol_request_svc_maker: ProtoHdlrMkr,
    our_basic_node_data: BasicNodeData,
    connection_parent_span: Span,
) -> Result<Client<Z>, HandshakeError>
where
    AdrBook: AddressBook<Z> + Clone,
    CSync: CoreSyncSvc + Clone,
    ProtoHdlrMkr: ProtocolRequestHandlerMaker<Z>,
    BrdcstStrm: Stream<Item = BroadcastMessage> + Send + 'static,
    BrdcstStrmMkr: Fn(InternalPeerID<Z::Addr>) -> BrdcstStrm + Send + 'static,
{
    let DoHandshakeRequest {
        addr,
        mut peer_stream,
        mut peer_sink,
        direction,
        permit,
    } = req;

    // A list of protocol messages the peer has sent during the handshake for us to handle after the handshake.
    // see: [`MAX_EAGER_PROTOCOL_MESSAGES`]
    let mut eager_protocol_messages = Vec::new();

    let (peer_core_sync, peer_node_data) = match direction {
        ConnectionDirection::Inbound => {
            // Inbound handshake the peer sends the request.
            tracing::debug!("waiting for handshake request.");

            let Message::Request(AdminRequestMessage::Handshake(handshake_req)) =
                wait_for_message::<Z>(
                    LevinCommand::Handshake,
                    true,
                    &mut peer_sink,
                    &mut peer_stream,
                    &mut eager_protocol_messages,
                    &our_basic_node_data,
                )
                .await?
            else {
                panic!("wait_for_message returned ok with wrong message.");
            };

            tracing::debug!("Received handshake request.");
            // We will respond to the handshake request later.
            (handshake_req.payload_data, handshake_req.node_data)
        }
        ConnectionDirection::Outbound => {
            // Outbound handshake, we send the request.
            send_hs_request::<Z, _>(
                &mut peer_sink,
                &mut core_sync_svc,
                our_basic_node_data.clone(),
            )
            .await?;

            // Wait for the handshake response.
            let Message::Response(AdminResponseMessage::Handshake(handshake_res)) =
                wait_for_message::<Z>(
                    LevinCommand::Handshake,
                    false,
                    &mut peer_sink,
                    &mut peer_stream,
                    &mut eager_protocol_messages,
                    &our_basic_node_data,
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

            // Tell our address book about the new peers.
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

    /*
    // monerod sends a request for support flags if the peer doesn't specify any but this seems unnecessary
    // as the peer should specify them in the handshake.

    if peer_node_data.support_flags.is_empty() {
        tracing::debug!(
            "Peer didn't send support flags or has no features, sending request to make sure."
        );
        peer_sink
            .send(Message::Request(RequestMessage::SupportFlags).into())
            .await?;

        let Message::Response(ResponseMessage::SupportFlags(support_flags_res)) =
            wait_for_message::<Z>(
                LevinCommand::SupportFlags,
                false,
                &mut peer_sink,
                &mut peer_stream,
                &mut eager_protocol_messages,
                &our_basic_node_data,
            )
            .await?
        else {
            panic!("wait_for_message returned ok with wrong message.");
        };

        tracing::debug!("Received support flag response.");
        peer_node_data.support_flags = support_flags_res.support_flags;
    }

    */

    // Make sure the pruning seed is valid.
    let pruning_seed = PruningSeed::decompress_p2p_rules(peer_core_sync.pruning_seed)?;

    // public_address, if Some, is the reachable address of the node.
    let public_address = 'check_out_addr: {
        match direction {
            ConnectionDirection::Inbound => {
                // First send the handshake response.
                send_hs_response::<Z, _, _>(
                    &mut peer_sink,
                    &mut core_sync_svc,
                    &mut address_book,
                    our_basic_node_data.clone(),
                )
                .await?;

                // Now if the peer specifies a reachable port, open a connection and ping them to check.
                if peer_node_data.my_port != 0 {
                    let InternalPeerID::KnownAddr(mut outbound_address) = addr else {
                        // Anonymity network, we don't know the inbound address.
                        break 'check_out_addr None;
                    };

                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "u32 does not make sense as a port so just truncate it."
                    )]
                    outbound_address.set_port(peer_node_data.my_port as u16);

                    let Ok(Ok(ping_peer_id)) = timeout(
                        PING_TIMEOUT,
                        ping::<Z>(outbound_address).instrument(info_span!("ping")),
                    )
                    .await
                    else {
                        // The ping was not successful.
                        break 'check_out_addr None;
                    };

                    // Make sure we are talking to the right node.
                    if ping_peer_id == peer_node_data.peer_id {
                        break 'check_out_addr Some(outbound_address);
                    }
                }
                // The peer did not specify a reachable port or the ping was not successful.
                None
            }
            ConnectionDirection::Outbound => {
                let InternalPeerID::KnownAddr(outbound_addr) = addr else {
                    unreachable!("How could we make an outbound connection to an unknown address");
                };

                // This is an outbound connection, this address is obviously reachable.
                Some(outbound_addr)
            }
        }
    };

    tracing::debug!("Handshake complete.");

    let (connection_guard, handle) = HandleBuilder::new().with_permit(permit).build();

    // Tell the address book about the new connection.
    address_book
        .ready()
        .await?
        .call(AddressBookRequest::NewConnection {
            internal_peer_id: addr,
            public_address,
            handle: handle.clone(),
            id: peer_node_data.peer_id,
            pruning_seed,
            rpc_port: peer_node_data.rpc_port,
            rpc_credits_per_hash: peer_node_data.rpc_credits_per_hash,
        })
        .await?;

    // Set up the connection data.
    let error_slot = SharedError::new();
    let (connection_tx, client_rx) = mpsc::channel(1);

    let info = PeerInformation {
        id: addr,
        handle,
        direction,
        pruning_seed,
        basic_node_data: peer_node_data,
        core_sync_data: Arc::new(Mutex::new(peer_core_sync)),
    };

    let protocol_request_handler = protocol_request_svc_maker
        .as_service()
        .ready()
        .await?
        .call(info.clone())
        .await?;

    let request_handler = PeerRequestHandler {
        address_book_svc: address_book.clone(),
        our_sync_svc: core_sync_svc.clone(),
        protocol_request_handler,
        our_basic_node_data,
        peer_info: info.clone(),
    };

    let connection = Connection::<Z, _, _, _, _>::new(
        peer_sink,
        client_rx,
        broadcast_stream_maker(addr),
        request_handler,
        connection_guard,
        error_slot.clone(),
    );

    let connection_span =
        tracing::error_span!(parent: &connection_parent_span, "connection", %addr);
    let connection_handle = tokio::spawn(
        connection
            .run(peer_stream.fuse(), eager_protocol_messages)
            .instrument(connection_span),
    );

    let semaphore = Arc::new(Semaphore::new(1));

    let timeout_handle = tokio::spawn(connection_timeout_monitor_task(
        info.clone(),
        connection_tx.clone(),
        Arc::clone(&semaphore),
        address_book,
        core_sync_svc,
    ));

    let client = Client::<Z>::new(
        info,
        connection_tx,
        connection_handle,
        timeout_handle,
        semaphore,
        error_slot,
    );

    Ok(client)
}

/// Sends a [`AdminRequestMessage::Handshake`] down the peer sink.
async fn send_hs_request<Z: NetworkZone, CSync>(
    peer_sink: &mut Z::Sink,
    core_sync_svc: &mut CSync,
    our_basic_node_data: BasicNodeData,
) -> Result<(), HandshakeError>
where
    CSync: CoreSyncSvc,
{
    let CoreSyncDataResponse(our_core_sync_data) = core_sync_svc
        .ready()
        .await?
        .call(CoreSyncDataRequest)
        .await?;

    let req = HandshakeRequest {
        node_data: our_basic_node_data,
        payload_data: our_core_sync_data,
    };

    tracing::debug!("Sending handshake request.");

    peer_sink
        .send(Message::Request(AdminRequestMessage::Handshake(req)).into())
        .await?;

    Ok(())
}

/// Sends a [`AdminResponseMessage::Handshake`] down the peer sink.
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
    let CoreSyncDataResponse(our_core_sync_data) = core_sync_svc
        .ready()
        .await?
        .call(CoreSyncDataRequest)
        .await?;

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
        .send(Message::Response(AdminResponseMessage::Handshake(res)).into())
        .await?;

    Ok(())
}

/// Waits for a message with a specific [`LevinCommand`].  
///
/// The message needed must not be a protocol message, only request/ response "admin" messages are allowed.
///
/// `levin_command` is the [`LevinCommand`] you need and `request` is for if the message is a request.
async fn wait_for_message<Z: NetworkZone>(
    levin_command: LevinCommand,
    request: bool,

    peer_sink: &mut Z::Sink,
    peer_stream: &mut Z::Stream,

    eager_protocol_messages: &mut Vec<cuprate_wire::ProtocolMessage>,

    our_basic_node_data: &BasicNodeData,
) -> Result<Message, HandshakeError> {
    let mut allow_support_flag_req = true;
    let mut allow_ping = true;

    while let Some(message) = peer_stream.next().await {
        let message = message?;

        match message {
            Message::Protocol(protocol_message) => {
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

                match req_message {
                    AdminRequestMessage::SupportFlags => {
                        if !allow_support_flag_req {
                            return Err(HandshakeError::PeerSentInvalidMessage(
                                "Peer sent 2 support flag requests",
                            ));
                        }
                        send_support_flags::<Z>(peer_sink, our_basic_node_data.support_flags)
                            .await?;
                        // don't let the peer send more after the first request.
                        allow_support_flag_req = false;
                        continue;
                    }
                    AdminRequestMessage::Ping => {
                        if !allow_ping {
                            return Err(HandshakeError::PeerSentInvalidMessage(
                                "Peer sent 2 ping requests",
                            ));
                        }

                        send_ping_response::<Z>(peer_sink, our_basic_node_data.peer_id).await?;

                        // don't let the peer send more after the first request.
                        allow_ping = false;
                        continue;
                    }
                    _ => {
                        return Err(HandshakeError::PeerSentInvalidMessage(
                            "Peer sent an admin request before responding to the handshake",
                        ));
                    }
                }
            }
            Message::Response(res_message) if !request => {
                if res_message.command() == levin_command {
                    return Ok(Message::Response(res_message));
                }

                tracing::debug!("Received unexpected response: {}", res_message.command());
                return Err(HandshakeError::PeerSentInvalidMessage(
                    "Peer sent an incorrect response",
                ));
            }

            Message::Response(_) => Err(HandshakeError::PeerSentInvalidMessage(
                "Peer sent an incorrect message",
            )),
        }?;
    }

    Err(BucketError::IO(std::io::Error::new(
        std::io::ErrorKind::ConnectionAborted,
        "The peer stream returned None",
    ))
    .into())
}

/// Sends a [`AdminResponseMessage::SupportFlags`] down the peer sink.
async fn send_support_flags<Z: NetworkZone>(
    peer_sink: &mut Z::Sink,
    support_flags: PeerSupportFlags,
) -> Result<(), HandshakeError> {
    tracing::debug!("Sending support flag response.");
    Ok(peer_sink
        .send(
            Message::Response(AdminResponseMessage::SupportFlags(SupportFlagsResponse {
                support_flags,
            }))
            .into(),
        )
        .await?)
}

/// Sends a [`AdminResponseMessage::Ping`] down the peer sink.
async fn send_ping_response<Z: NetworkZone>(
    peer_sink: &mut Z::Sink,
    peer_id: u64,
) -> Result<(), HandshakeError> {
    tracing::debug!("Sending ping response.");
    Ok(peer_sink
        .send(
            Message::Response(AdminResponseMessage::Ping(PingResponse {
                status: PING_OK_RESPONSE_STATUS_TEXT,
                peer_id,
            }))
            .into(),
        )
        .await?)
}

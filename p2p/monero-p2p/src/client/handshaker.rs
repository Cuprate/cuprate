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
    BasicNodeData, BucketError, CoreSyncData, LevinCommand, Message, RequestMessage,
    ResponseMessage,
};

use crate::{
    AddressBook, AddressBookRequest, AddressBookResponse, ConnectionDirection, CoreSyncDataRequest,
    CoreSyncDataResponse, CoreSyncSvc, MessageID, NetworkZone, PeerRequestHandler,
    MAX_PEERS_IN_PEER_LIST_MESSAGE,
};

const MAX_EAGER_PROTOCOL_MESSAGES: usize = 2;

#[derive(Debug, thiserror::Error)]
pub enum HandshakeError {
    #[error("peer has the same node ID as us")]
    PeerHasSameNodeID,
    #[error("peer is on a different network")]
    IncorrectNetwork,
    #[error("peer sent a peer list with peers from different zones")]
    PeerSentIncorrectPeerList(#[from] crate::services::PeerListConversionError),
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

        async move {
            // TODO: timeouts
            handshake(
                addr,
                peer_stream,
                peer_sink,
                direction,
                address_book,
                core_sync_svc,
                peer_request_svc,
                our_basic_node_data,
            )
            .await
        }
        .instrument(span)
        .boxed()
    }
}

#[allow(clippy::too_many_arguments)]
async fn handshake<Z: NetworkZone, AdrBook, CSync, ReqHdlr>(
    addr: Z::Addr,
    mut peer_stream: Z::Stream,
    mut peer_sink: Z::Sink,
    direction: ConnectionDirection,
    mut address_book: AdrBook,
    mut core_sync_svc: CSync,
    peer_request_svc: ReqHdlr,
    our_basic_node_data: BasicNodeData,
) -> Result<(), HandshakeError>
where
    AdrBook: AddressBook<Z>,
    CSync: CoreSyncSvc,
    ReqHdlr: PeerRequestHandler,
{
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
        .call(CoreSyncDataRequest::HandleIncoming(peer_core_sync))
        .await?;

    tracing::debug!("Handshake complete.");

    Ok(())
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

async fn wait_for_message<Z: NetworkZone>(
    levin_command: LevinCommand,
    request: bool,
    peer_sink: &mut Z::Sink,
    peer_stream: &mut Z::Stream,
    eager_protocol_messages: &mut Vec<monero_wire::ProtocolMessage>,
    allow_support_flag_req: &mut bool,
    support_flags: PeerSupportFlags,
) -> Result<Message, HandshakeError> {
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
                        "Peer sent too many protocl messages before a handshake response."
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
                    "Peer sent a admin request before responding to the handshake",
                ));
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

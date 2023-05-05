use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use futures::FutureExt;
use futures::{channel::mpsc, AsyncRead, AsyncWrite, SinkExt, StreamExt};
use monero_wire::messages::admin::{SupportFlagsRequest, SupportFlagsResponse};
use monero_wire::messages::MessageRequest;
use thiserror::Error;
use tokio::time;
use tower::{BoxError, Service, ServiceExt};

use crate::address_book::{AddressBookError, AddressBookRequest, AddressBookResponse};
use crate::connection_counter::ConnectionTracker;
use crate::constants::{HANDSHAKE_TIMEOUT, P2P_MAX_PEERS_IN_HANDSHAKE};
use crate::protocol::{
    CoreSyncDataRequest, CoreSyncDataResponse, Direction, InternalMessageRequest,
    InternalMessageResponse,
};
use crate::{Config, NetZoneBasicNodeData};
use cuprate_common::{HardForks, Network, PruningSeed};
use monero_wire::{
    levin::{BucketError, MessageSink, MessageStream},
    messages::{
        admin::{HandshakeRequest, HandshakeResponse},
        common::PeerSupportFlags,
        BasicNodeData, CoreSyncData, MessageResponse, PeerID, PeerListEntryBase,
    },
    Message, NetworkAddress,
};
use tracing::Instrument;

use super::client::Client;
use super::{
    client::ConnectionInfo,
    connection::{ClientRequest, Connection},
    PeerError,
};

#[derive(Debug, Error)]
pub enum HandShakeError {
    #[error("The peer did not complete the handshake fast enough")]
    PeerTimedOut,
    #[error("The peer has a weird pruning scheme")]
    PeerClaimedWeirdPruning,
    #[error("The peer has an unexpected top version")]
    PeerHasUnexpectedTopVersion,
    #[error("The peer does not have the minimum support flags")]
    PeerDoesNotHaveTheMinimumSupportFlags,
    #[error("The peer is on a different network")]
    PeerIsOnADifferentNetwork,
    #[error("The peer sent too many peers, considered spamming")]
    PeerSentTooManyPeers,
    #[error("The peer sent a wrong response to our handshake")]
    PeerSentWrongResponse,
    #[error("Bucket error while communicating with peer: {0}")]
    BucketError(#[from] BucketError),
}

pub struct DoHandshakeRequest<W, R> {
    pub read: R,
    pub write: W,
    pub direction: Direction,
    pub addr: NetworkAddress,
    pub connection_tracker: ConnectionTracker,
}

#[derive(Debug, Clone)]
pub struct Handshaker<Svc, CoreSync, AdrBook> {
    basic_node_data: NetZoneBasicNodeData,
    network: Network,
    parent_span: tracing::Span,
    address_book: AdrBook,
    core_sync_svc: CoreSync,
    peer_request_service: Svc,
}

impl<Svc, CoreSync, AdrBook> Handshaker<Svc, CoreSync, AdrBook> {
    pub fn new(
        basic_node_data: NetZoneBasicNodeData,
        network: Network,
        address_book: AdrBook,
        core_sync_svc: CoreSync,
        peer_request_service: Svc,
    ) -> Self {
        Handshaker {
            basic_node_data,
            network,
            parent_span: tracing::Span::current(),
            address_book,
            core_sync_svc,
            peer_request_service,
        }
    }
}

impl<Svc, CoreSync, AdrBook, W, R> tower::Service<DoHandshakeRequest<W, R>>
    for Handshaker<Svc, CoreSync, AdrBook>
where
    CoreSync: Service<CoreSyncDataRequest, Response = CoreSyncDataResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    CoreSync::Future: Send,

    Svc: Service<InternalMessageRequest, Response = InternalMessageResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    Svc::Future: Send,

    AdrBook: Service<AddressBookRequest, Response = AddressBookResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    AdrBook::Future: Send,

    W: AsyncWrite + std::marker::Unpin + Send + 'static,
    R: AsyncRead + std::marker::Unpin + Send + 'static,
{
    type Error = BoxError;
    type Response = Client;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: DoHandshakeRequest<W, R>) -> Self::Future {
        let DoHandshakeRequest {
            read,
            write,
            direction,
            addr,
            connection_tracker,
        } = req;

        let peer_stream = MessageStream::new(read);
        let peer_sink = MessageSink::new(write);

        let span = tracing::debug_span!("Handshaker");

        let connection_span = tracing::debug_span!(parent: &self.parent_span, "Connection");

        let core_sync_svc = self.core_sync_svc.clone();
        let address_book = self.address_book.clone();
        let peer_request_service = self.peer_request_service.clone();

        let state_machine = HandshakeSM {
            peer_sink,
            peer_stream,
            direction,
            addr,
            network: self.network,
            basic_node_data: match addr.get_zone() {
                monero_wire::NetZone::Public => self.basic_node_data.public.clone(),
                _ => todo!(),
            },
            address_book,
            core_sync_svc,
            peer_request_service,
            connection_span,
            connection_tracker,
            state: HandshakeState::Start,
        };

        let ret = time::timeout(HANDSHAKE_TIMEOUT, state_machine.do_handshake());

        async move {
            match ret.await {
                Ok(handshake) => handshake,
                Err(_) => Err(HandShakeError::PeerTimedOut.into()),
            }
        }
        .instrument(span)
        .boxed()
    }
}

enum HandshakeState {
    Start,
    WaitingForHandshakeResponse,
    WaitingForSupportFlagResponse(BasicNodeData, CoreSyncData),
    Complete(BasicNodeData, CoreSyncData),
}

impl HandshakeState {
    pub fn is_complete(&self) -> bool {
        matches!(self, Complete(_))
    }

    pub fn peer_data(self) -> Option<(BasicNodeData, CoreSyncData)> {
        match self {
            HandshakeState::Complete(bnd, coresync) => Some((bnd, coresync)),
            _ => None,
        }
    }
}

struct HandshakeSM<Svc, CoreSync, AdrBook, W, R> {
    peer_sink: MessageSink<W, Message>,
    peer_stream: MessageStream<R, Message>,
    direction: Direction,
    addr: NetworkAddress,
    network: Network,

    basic_node_data: BasicNodeData,
    address_book: AdrBook,
    core_sync_svc: CoreSync,
    peer_request_service: Svc,

    connection_span: tracing::Span,
    connection_tracker: ConnectionTracker,

    state: HandshakeState,
}

impl<Svc, CoreSync, AdrBook, W, R> HandshakeSM<Svc, CoreSync, AdrBook, W, R>
where
    CoreSync: Service<CoreSyncDataRequest, Response = CoreSyncDataResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    CoreSync::Future: Send,

    Svc: Service<InternalMessageRequest, Response = InternalMessageResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    Svc::Future: Send,

    AdrBook: Service<AddressBookRequest, Response = AddressBookResponse, Error = BoxError>
        + Clone
        + Send
        + 'static,
    AdrBook::Future: Send,

    W: AsyncWrite + std::marker::Unpin + Send + 'static,
    R: AsyncRead + std::marker::Unpin + Send + 'static,
{
    async fn get_our_core_sync(&mut self) -> Result<CoreSyncData, BoxError> {
        let core_sync_svc = self.core_sync_svc.ready().await?;
        let CoreSyncDataResponse(core_sync) = core_sync_svc.call(CoreSyncDataRequest).await?;
        tracing::trace!("Got core sync data: {core_sync:?}");
        Ok(core_sync)
    }

    async fn send_handshake_req(
        &mut self,
        node_data: BasicNodeData,
        payload_data: CoreSyncData,
    ) -> Result<(), HandShakeError> {
        let handshake_req = HandshakeRequest {
            node_data,
            payload_data,
        };

        tracing::trace!("Sending handshake request: {handshake_req:?}");

        let message: Message = Message::Request(handshake_req.into());
        self.peer_sink.send(message).await?;
        Ok(())
    }

    async fn get_handshake_res(&mut self) -> Result<HandshakeResponse, HandShakeError> {
        // put a timeout on this
        let Message::Response(MessageResponse::Handshake(handshake_res)) =  self.peer_stream.next().await.expect("MessageSink will not return None")? else {
            return Err(HandShakeError::PeerSentWrongResponse);
        };

        tracing::trace!("Received handshake response: {handshake_res:?}");

        Ok(handshake_res)
    }

    async fn send_support_flag_req(&mut self) -> Result<(), HandShakeError> {
        tracing::trace!("Peer sent no support flags, sending request");

        let message: Message = Message::Request(SupportFlagsRequest.into());
        self.peer_sink.send(message).await?;

        Ok(())
    }

    async fn handle_handshake_response(&mut self, res: HandshakeResponse) -> Result<(), BoxError> {
        let HandshakeResponse {
            node_data: peer_node_data,
            payload_data: peer_core_sync,
            local_peerlist_new,
        } = res;

        if peer_node_data.network_id != self.network.network_id() {
            tracing::debug!("Handshake failed: peer is on a different network");
            return Err(HandShakeError::PeerIsOnADifferentNetwork.into());
        }

        if local_peerlist_new.len() > P2P_MAX_PEERS_IN_HANDSHAKE {
            tracing::debug!("Handshake failed: peer sent too many peers in response");
            return Err(HandShakeError::PeerSentTooManyPeers.into());
        }

        // Tell the address book about the new peers
        self.address_book
            .ready()
            .await?
            .call(AddressBookRequest::HandleNewPeerList(
                local_peerlist_new,
                self.addr.get_zone(),
            ))
            .await?;

        // coresync, pruning seed

        if peer_node_data.support_flags.is_empty() {
            self.send_support_flag_req().await?;
            self.state =
                HandshakeState::WaitingForSupportFlagResponse(peer_node_data, CoreSyncData);
        } else {
            self.state = HandshakeState::Complete(peer_node_data, CoreSyncData);
        }

        Ok(())
    }

    async fn handle_message_response(&mut self, response: MessageResponse) -> Result<(), BoxError> {
        match (&mut self.state, response) {
            (
                HandshakeState::WaitingForHandshakeResponse,
                MessageResponse::Handshake(handshake),
            ) => self.handle_handshake_response(handshake).await,
            (
                HandshakeState::WaitingForSupportFlagResponse(bnd, coresync),
                MessageResponse::SupportFlags(support_flags),
            ) => {
                bnd.support_flags = support_flags.support_flags;
                self.state = HandshakeState::Complete(bnd.clone(), coresync.clone());
                Ok(())
            }
            _ => Err(HandShakeError::PeerSentWrongResponse.into()),
        }
    }

    async fn send_support_flags(
        &mut self,
        support_flags: PeerSupportFlags,
    ) -> Result<(), HandShakeError> {
        let message = Message::Response(SupportFlagsResponse { support_flags }.into());
        self.peer_sink.send(message).await?;
        Ok(())
    }

    async fn do_outbound_handshake(&mut self) -> Result<(), BoxError> {
        let core_sync = self.get_our_core_sync().await?;
        self.send_handshake_req(self.basic_node_data.clone(), core_sync)
            .await?;
        self.state = HandshakeState::WaitingForHandshakeResponse;

        while !self.state.is_complete() {
            match self.peer_stream.next().await {
                Some(mes) => {
                    let mes = mes?;
                    match mes {
                        Message::Request(MessageRequest::SupportFlags(_)) => {
                            self.send_support_flags(self.basic_node_data.support_flags)
                                .await?
                        }
                        Message::Response(response) => {
                            self.handle_message_response(response).await?
                        }
                        _ => return Err(HandShakeError::PeerSentWrongResponse.into()),
                    }
                }
                None => unreachable!("peer_stream wont return None"),
            }
        }

        Ok(())
    }

    async fn do_handshake(mut self) -> Result<Client, BoxError> {
        match self.direction {
            Direction::Outbound => self.do_outbound_handshake().await?,
            Direction::Inbound => todo!(),
        }

        let (server_tx, server_rx) = mpsc::channel(3);

        let (peer_node_data, coresync) = self
            .state
            .peer_data()
            .expect("We must be in state complete to be here");

        let pruning_seed = PruningSeed::try_from(coresync.pruning_seed).map_err(Into::into)?;

        let peer_height = AtomicU64::new(coresync.current_height).into();

        let connection_info = ConnectionInfo {
            addr: self.addr,
            support_flags: peer_node_data.support_flags,
            pruning_seed,
            peer_height: peer_height_cumm_diff.clone(),
            peer_id: peer_node_data.peer_id,
            rpc_port: peer_node_data.rpc_port,
            rpc_credits_per_hash: peer_node_data.rpc_credits_per_hash,
        };

        let connection = Connection::new(
            self.addr,
            self.peer_sink,
            self.peer_stream,
            peer_height,
            server_rx,
            self.connection_tracker,
            self.peer_request_service,
        );

        let client = Client::new(connection_info.into(), server_tx);

        tokio::task::spawn(connection.run().instrument(self.connection_span));

        Ok(client)
    }
}

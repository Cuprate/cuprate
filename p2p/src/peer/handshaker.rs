use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures::FutureExt;
use futures::{channel::mpsc, AsyncRead, AsyncWrite, SinkExt, StreamExt};
use monero_wire::messages::admin::{SupportFlagsRequest, SupportFlagsResponse};
use monero_wire::messages::MessageRequest;
use thiserror::Error;
use tokio::time;
use tower::{Service, ServiceExt};

use crate::address_book::{AddressBookError, AddressBookRequest, AddressBookResponse};
use crate::protocol::temp_database::{DataBaseRequest, DataBaseResponse, DatabaseError};
use crate::protocol::{
    Direction, InternalMessageRequest, InternalMessageResponse, P2P_MAX_PEERS_IN_HANDSHAKE,
};
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
    connection::{ClientRequest, Connection, PeerSyncChange},
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
    #[error("Address book err: {0}")]
    AddressBookError(#[from] AddressBookError),
    #[error("The peer sent too many peers, considered spamming")]
    PeerSentTooManyPeers,
    #[error("The peer sent a wrong response to our handshake")]
    PeerSentWrongResponse,
    #[error("The syncer returned an error")]
    DataBaseError(#[from] DatabaseError),
    #[error("Bucket error while communicating with peer: {0}")]
    BucketError(#[from] BucketError),
}

pub struct NetworkConfig {
    /// Port
    my_port: u32,
    /// The Network
    network: Network,
    /// Peer ID
    peer_id: PeerID,
    /// RPC Port
    rpc_port: u16,
    /// RPC Credits Per Hash
    rpc_credits_per_hash: u32,
    our_support_flags: PeerSupportFlags,
    minimum_peer_support_flags: PeerSupportFlags,
    handshake_timeout: time::Duration,
    max_in_peers: u32,
    target_out_peers: u32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        NetworkConfig {
            my_port: 18080,
            network: Network::MainNet,
            peer_id: PeerID(21),
            rpc_port: 0,
            rpc_credits_per_hash: 0,
            our_support_flags: PeerSupportFlags::get_support_flag_fluffy_blocks(),
            minimum_peer_support_flags: PeerSupportFlags::from(0_u32),
            handshake_timeout: time::Duration::from_secs(5),
            max_in_peers: 13,
            target_out_peers: 21,
        }
    }
}

impl NetworkConfig {
    pub fn basic_node_data(&self) -> BasicNodeData {
        BasicNodeData {
            my_port: self.my_port,
            network_id: self.network.network_id(),
            peer_id: self.peer_id,
            support_flags: self.our_support_flags,
            rpc_port: self.rpc_port,
            rpc_credits_per_hash: self.rpc_credits_per_hash,
        }
    }
}

pub struct Handshake<W, R> {
    sink: MessageSink<W, Message>,
    stream: MessageStream<R, Message>,
    direction: Direction,
    addr: NetworkAddress,
}

pub struct Handshaker<Bc, Svc, AdrBook> {
    config: NetworkConfig,
    parent_span: tracing::Span,
    address_book: AdrBook,
    blockchain: Bc,
    peer_sync_states: mpsc::Sender<PeerSyncChange>,
    peer_request_service: Svc,
}

impl<Bc, Svc, AdrBook, W, R> tower::Service<Handshake<W, R>> for Handshaker<Bc, Svc, AdrBook>
where
    Bc: Service<DataBaseRequest, Response = DataBaseResponse, Error = DatabaseError>
        + Clone
        + Send
        + 'static,
    Bc::Future: Send,

    Svc: Service<InternalMessageRequest, Response = InternalMessageResponse, Error = PeerError>
        + Clone
        + Send
        + 'static,
    Svc::Future: Send,

    AdrBook: Service<AddressBookRequest, Response = AddressBookResponse, Error = AddressBookError>
        + Clone
        + Send
        + 'static,
    AdrBook::Future: Send,

    W: AsyncWrite + std::marker::Unpin + Send + 'static,
    R: AsyncRead + std::marker::Unpin + Send + 'static,
{
    type Error = HandShakeError;
    type Response = Client;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Handshake<W, R>) -> Self::Future {
        let Handshake {
            sink: mut peer_sink,
            stream: mut peer_stream,
            direction,
            addr,
        } = req;

        let span = tracing::debug_span!("Handshaker");

        let connection_span = tracing::debug_span!(parent: &self.parent_span, "Connection");

        let blockchain = self.blockchain.clone();
        let address_book = self.address_book.clone();
        let syncer_tx = self.peer_sync_states.clone();
        let peer_request_service = self.peer_request_service.clone();

        let state_machine = HandshakeSM {
            peer_sink,
            peer_stream,
            direction,
            addr,
            network: self.config.network,
            basic_node_data: self.config.basic_node_data(),
            minimum_support_flags: self.config.minimum_peer_support_flags,
            address_book,
            blockchain,
            peer_request_service,
            connection_span,
            state: HandshakeState::Start,
        };

        let ret = time::timeout(self.config.handshake_timeout, state_machine.do_handshake());

        async move {
            match ret.await {
                Ok(handshake) => handshake,
                Err(_) => Err(HandShakeError::PeerTimedOut),
            }
        }
        .boxed()
    }
}

enum HandshakeState {
    Start,
    WaitingForHandshakeResponse,
    WaitingForSupportFlagResponse(BasicNodeData),
    Complete(BasicNodeData),
}

impl HandshakeState {
    pub fn is_complete(&self) -> bool {
        matches!(self, HandshakeState::Complete(_))
    }

    pub fn peer_basic_node_data(self) -> Option<BasicNodeData> {
        match self {
            HandshakeState::Complete(sup) => Some(sup),
            _ => None,
        }
    }
}

struct HandshakeSM<Bc, Svc, AdrBook, W, R> {
    peer_sink: MessageSink<W, Message>,
    peer_stream: MessageStream<R, Message>,
    direction: Direction,
    addr: NetworkAddress,
    network: Network,

    basic_node_data: BasicNodeData,
    minimum_support_flags: PeerSupportFlags,
    address_book: AdrBook,
    blockchain: Bc,
    peer_request_service: Svc,
    connection_span: tracing::Span,

    state: HandshakeState,
}

impl<Bc, Svc, AdrBook, W, R> HandshakeSM<Bc, Svc, AdrBook, W, R>
where
    Bc: Service<DataBaseRequest, Response = DataBaseResponse, Error = DatabaseError>
        + Clone
        + Send
        + 'static,
    Bc::Future: Send,

    Svc: Service<InternalMessageRequest, Response = InternalMessageResponse, Error = PeerError>
        + Clone
        + Send
        + 'static,
    Svc::Future: Send,

    AdrBook: Service<AddressBookRequest, Response = AddressBookResponse, Error = AddressBookError>
        + Clone
        + Send
        + 'static,
    AdrBook::Future: Send,

    W: AsyncWrite + std::marker::Unpin + Send + 'static,
    R: AsyncRead + std::marker::Unpin + Send + 'static,
{
    async fn get_our_core_sync(&mut self) -> Result<CoreSyncData, DatabaseError> {
        let blockchain = self.blockchain.ready().await?;
        let DataBaseResponse::CoreSyncData(core_sync) = blockchain.call(DataBaseRequest::CoreSyncData).await? else {
            unreachable!("Database will always return the requested item")
        };
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

    async fn handle_handshake_response(
        &mut self,
        res: HandshakeResponse,
    ) -> Result<(), HandShakeError> {
        let HandshakeResponse {
            node_data: peer_node_data,
            payload_data: peer_core_sync,
            local_peerlist_new,
        } = res;

        if !peer_node_data
            .support_flags
            .contains(&self.minimum_support_flags)
        {
            tracing::debug!("Handshake failed: peer does not have minimum support flags");
            return Err(HandShakeError::PeerDoesNotHaveTheMinimumSupportFlags);
        }

        if peer_node_data.network_id != self.network.network_id() {
            tracing::debug!("Handshake failed: peer is on a different network");
            return Err(HandShakeError::PeerIsOnADifferentNetwork);
        }

        if local_peerlist_new.len() > P2P_MAX_PEERS_IN_HANDSHAKE {
            tracing::debug!("Handshake failed: peer sent too many peers in response");
            return Err(HandShakeError::PeerSentTooManyPeers);
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
            self.state = HandshakeState::WaitingForSupportFlagResponse(peer_node_data);
        } else {
            self.state = HandshakeState::Complete(peer_node_data);
        }

        Ok(())
    }

    async fn handle_message_response(
        &mut self,
        response: MessageResponse,
    ) -> Result<(), HandShakeError> {
        match (&mut self.state, response) {
            (
                HandshakeState::WaitingForHandshakeResponse,
                MessageResponse::Handshake(handshake),
            ) => self.handle_handshake_response(handshake).await,
            (
                HandshakeState::WaitingForSupportFlagResponse(bnd),
                MessageResponse::SupportFlags(support_flags),
            ) => {
                bnd.support_flags = support_flags.support_flags;
                self.state = HandshakeState::Complete(bnd.clone());
                Ok(())
            }
            _ => Err(HandShakeError::PeerSentWrongResponse),
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

    async fn do_outbound_handshake(&mut self) -> Result<(), HandShakeError> {
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
                        _ => return Err(HandShakeError::PeerSentWrongResponse),
                    }
                }
                None => unreachable!("peer_stream wont return None"),
            }
        }

        Ok(())
    }

    async fn do_handshake(mut self) -> Result<Client, HandShakeError> {
        match self.direction {
            Direction::Outbound => self.do_outbound_handshake().await?,
            Direction::Inbound => todo!(),
        }

        let (server_tx, server_rx) = mpsc::channel(3);

        let (replace_me, replace_me_rx) = mpsc::channel(3);

        let peer_node_data = self
            .state
            .peer_basic_node_data()
            .expect("We must be in state complete to be here");
        let connection_info = ConnectionInfo {
            addr: self.addr,
            support_flags: peer_node_data.support_flags,
            peer_id: peer_node_data.peer_id,
            rpc_port: peer_node_data.rpc_port,
            rpc_credits_per_hash: peer_node_data.rpc_credits_per_hash,
        };

        let connection = Connection::new(
            self.addr,
            self.peer_sink,
            self.peer_stream,
            server_rx,
            replace_me,
            self.peer_request_service,
        );

        let client = Client::new(connection_info.into(), server_tx);

        tokio::task::spawn(connection.run().instrument(self.connection_span));

        Ok(client)
    }
}

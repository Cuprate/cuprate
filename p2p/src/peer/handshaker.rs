/// This module contains the logic for turning [`AsyncRead`] and [`AsyncWrite`]
/// into [`Client`] and [`Connection`].
///
/// The main entry point is modeled as a [`tower::Service`] the struct being
/// [`Handshaker`]. The [`Handshaker`] accepts handshake requests: [`DoHandshakeRequest`]
/// and creates a state machine that's drives the handshake forward: [`HandshakeSM`] and
/// eventually outputs a [`Client`] and [`Connection`].
///
use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;

use futures::FutureExt;
use futures::{channel::mpsc, AsyncRead, AsyncWrite, SinkExt, StreamExt};
use thiserror::Error;
use tokio::time;
use tower::{BoxError, Service, ServiceExt};
use tracing::Instrument;

use cuprate_common::{Network, PruningSeed};
use monero_wire::messages::admin::{SupportFlagsRequest, SupportFlagsResponse};
use monero_wire::messages::TimedSync;
use monero_wire::messages::{AdminMessage, MessageRequest};
use monero_wire::{
    levin::{BucketError, MessageSink, MessageStream},
    messages::{
        admin::{HandshakeRequest, HandshakeResponse},
        common::PeerSupportFlags,
        BasicNodeData, CoreSyncData, MessageResponse, PeerID, PeerListEntryBase,
    },
    Message, NetZone, NetworkAddress,
};

use super::{
    client::{Client, ConnectionInfo},
    connection::Connection,
    PeerError,
};
use crate::address_book::connection_handle::new_address_book_connection_handle;
use crate::address_book::{AddressBookRequest, AddressBookResponse};
use crate::connection_tracker::ConnectionTracker;
use crate::constants::{
    CUPRATE_MINIMUM_SUPPORT_FLAGS, HANDSHAKE_TIMEOUT, P2P_MAX_PEERS_IN_HANDSHAKE,
};
use crate::protocol::{
    CoreSyncDataRequest, CoreSyncDataResponse, Direction, InternalMessageRequest,
    InternalMessageResponse,
};
use crate::NetZoneBasicNodeData;

/// Possible handshake errors
#[derive(Debug, Error)]
pub enum HandShakeError {
    /// The peer did not complete the handshake fast enough.
    #[error("The peer did not complete the handshake fast enough")]
    PeerTimedOut,
    /// The Peer has non-standard pruning.
    #[error("The peer has a weird pruning scheme")]
    PeerClaimedWeirdPruning,
    /// The peer does not have the minimum support flags
    #[error("The peer does not have the minimum support flags")]
    PeerDoesNotHaveTheMinimumSupportFlags,
    /// The peer is not on the network we are on (MAINNET|TESTNET|STAGENET)
    #[error("The peer is on a different network")]
    PeerIsOnADifferentNetwork,
    /// The peer sent us too many peers, more than [`P2P_MAX_PEERS_IN_HANDSHAKE`]
    #[error("The peer sent too many peers, considered spamming")]
    PeerSentTooManyPeers,
    /// The peer sent an incorrect response
    #[error("The peer sent a wrong response to our handshake")]
    PeerSentWrongResponse,
    /// Error communicating with peer
    #[error("Bucket error while communicating with peer: {0}")]
    BucketError(#[from] BucketError),
}

/// An address used to connect to a peer.
#[derive(Debug, Copy, Clone)]
pub enum ConnectionAddr {
    /// Outbound connection to another peer.
    OutBound { address: NetworkAddress },
    /// An inbound direct connection to our node.
    InBoundDirect { ip_address: IpAddr },
    /// An inbound connection through a hidden network
    /// like Tor/ I2p
    InBoundProxy { net_zone: NetZone },
}

impl ConnectionAddr {
    /// Gets the [`NetworkAddress`] of this connection.
    pub fn get_network_address(&self, port: u16) -> Option<NetworkAddress> {
        match self {
            ConnectionAddr::OutBound { address } => Some(*address),
            ConnectionAddr::InBoundDirect { ip_address } => {
                Some(NetworkAddress::from_ip_port(*ip_address, port))
            }
            _ => None,
        }
    }
    /// Gets the [`NetZone`] of this connection.
    pub fn get_zone(&self) -> NetZone {
        match self {
            ConnectionAddr::OutBound { address } => address.get_zone(),
            ConnectionAddr::InBoundDirect { .. } => NetZone::Public,
            ConnectionAddr::InBoundProxy { net_zone } => *net_zone,
        }
    }

    /// Gets the [`Direction`] of this connection.
    pub fn direction(&self) -> Direction {
        match self {
            ConnectionAddr::OutBound { .. } => Direction::Outbound,
            ConnectionAddr::InBoundDirect { .. } | ConnectionAddr::InBoundProxy { .. } => {
                Direction::Inbound
            }
        }
    }
}

/// A request to handshake with a peer.
pub struct DoHandshakeRequest<W, R> {
    /// The read-half of the connection.
    pub read: R,
    /// The write-half of the connection.
    pub write: W,
    /// The [`ConnectionAddr`] of this connection.
    pub addr: ConnectionAddr,
    /// The [`ConnectionTracker`] of this connection.
    pub connection_tracker: ConnectionTracker,
}

/// A [`Service`] that accepts [`DoHandshakeRequest`] and
/// produces a [`Client`] and [`Connection`].
#[derive(Debug, Clone)]
pub struct Handshaker<Svc, CoreSync, AdrBook> {
    /// A collection of our [`BasicNodeData`] for each [`NetZone`]
    /// for more info see: [`NetZoneBasicNodeData`]
    basic_node_data: NetZoneBasicNodeData,
    /// The [`Network`] our node is using
    network: Network,
    /// The span [`Connection`] tasks will be [`tracing::instrument`]ed with
    parent_span: tracing::Span,
    /// The address book [`Service`]
    address_book: AdrBook,
    /// A [`Service`] to handle incoming [`CoreSyncData`] and to get
    /// our [`CoreSyncData`].
    core_sync_svc: CoreSync,
    /// A service given to the [`Connection`] task to answer incoming
    /// requests to our node.
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

impl<Svc, CoreSync, AdrBook, W, R> Service<DoHandshakeRequest<W, R>>
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

    W: AsyncWrite + Unpin + Send + 'static,
    R: AsyncRead + Unpin + Send + 'static,
{
    type Response = Client;
    type Error = BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        // We are always ready.
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: DoHandshakeRequest<W, R>) -> Self::Future {
        let DoHandshakeRequest {
            read,
            write,
            addr,
            connection_tracker,
        } = req;

        // create the levin message stream/ sink.
        let peer_stream = MessageStream::new(read);
        let peer_sink = MessageSink::new(write);

        // The span the handshake state machine will use
        let span = tracing::debug_span!("Handshaker");

        // The span the connection task will use.
        let connection_span = tracing::debug_span!(parent: &self.parent_span, "Connection");

        // clone the services that the handshake state machine will need.
        let core_sync_svc = self.core_sync_svc.clone();
        let address_book = self.address_book.clone();
        let peer_request_service = self.peer_request_service.clone();

        let state_machine = HandshakeSM {
            peer_sink,
            peer_stream,
            addr,
            network: self.network,
            basic_node_data: self.basic_node_data.basic_node_data(&addr.get_zone()),
            address_book,
            core_sync_svc,
            peer_request_service,
            connection_span,
            connection_tracker,
            state: HandshakeState::Start,
        };
        // although callers should use a timeout do one here as well just to be safe.
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

/// The states a handshake can be in.
enum HandshakeState {
    /// The initial state.
    /// if this is an inbound handshake then this state means we
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

/// The state machine that drives a handshake forward and
/// accepts requests (that can happen during a handshake)
/// from a peer.
struct HandshakeSM<Svc, CoreSync, AdrBook, W, R> {
    /// The levin [`MessageSink`] for the peer.
    peer_sink: MessageSink<W, Message>,
    /// The levin [`MessageStream`] for the peer.
    peer_stream: MessageStream<R, Message>,
    /// The [`ConnectionAddr`] for the peer.
    addr: ConnectionAddr,
    /// The [`Network`] we are on.
    network: Network,

    /// Our [`BasicNodeData`].
    basic_node_data: BasicNodeData,
    /// The address book [`Service`]
    address_book: AdrBook,
    /// The core sync [`Service`] to handle incoming
    /// [`CoreSyncData`] and to retrieve ours.
    core_sync_svc: CoreSync,
    /// The [`Service`] passed to the [`Connection`]
    /// task to handle incoming peer requests.
    peer_request_service: Svc,

    /// The [`tracing::Span`] the [`Connection`] task
    /// will be [`tracing::instrument`]ed with.
    connection_span: tracing::Span,
    /// A connection tracker to keep track of the
    /// number of connections Cuprate is making.
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

    W: AsyncWrite + Unpin + Send + 'static,
    R: AsyncRead + Unpin + Send + 'static,
{
    /// Gets our [`CoreSyncData`] from the `core_sync_svc`.
    async fn get_our_core_sync(&mut self) -> Result<CoreSyncData, BoxError> {
        let core_sync_svc = self.core_sync_svc.ready().await?;
        let CoreSyncDataResponse::Ours(core_sync) = core_sync_svc.call(CoreSyncDataRequest::GetOurs).await? else {
            unreachable!("The Service must give correct responses");
        };
        tracing::trace!("Got core sync data: {core_sync:?}");
        Ok(core_sync)
    }

    /// Sends a [`HandshakeRequest`] to the peer.
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

    /// Sends a [`SupportFlagsRequest`] to the peer.
    /// This is done when a peer sends  no support flags in their
    /// [`HandshakeRequest`] or [`HandshakeResponse`].
    ///
    /// *note because Cuprate has minimum required support flags this won't
    /// happeen but is included here just in case this changes.
    async fn send_support_flag_req(&mut self) -> Result<(), HandShakeError> {
        tracing::trace!("Peer sent no support flags, sending request");

        let message: Message = Message::Request(SupportFlagsRequest.into());
        self.peer_sink.send(message).await?;

        Ok(())
    }

    /// Handles an incoming [`HandshakeResponse`].
    async fn handle_handshake_response(&mut self, res: HandshakeResponse) -> Result<(), BoxError> {
        let HandshakeResponse {
            node_data: peer_node_data,
            payload_data: peer_core_sync,
            local_peerlist_new,
        } = res;

        // Check the peer is on the correct network.
        if peer_node_data.network_id != self.network.network_id() {
            tracing::debug!("Handshake failed: peer is on a different network");
            return Err(HandShakeError::PeerIsOnADifferentNetwork.into());
        }

        // Check the peer meets the minimum support flags.
        if !peer_node_data
            .support_flags
            .contains(&CUPRATE_MINIMUM_SUPPORT_FLAGS)
        {
            tracing::debug!("Handshake failed: peer does not have minimum required support flags");
            return Err(HandShakeError::PeerDoesNotHaveTheMinimumSupportFlags.into());
        }

        // Check the peer didn't send too many peers.
        if local_peerlist_new.len() > P2P_MAX_PEERS_IN_HANDSHAKE {
            tracing::debug!("Handshake failed: peer sent too many peers in response");
            return Err(HandShakeError::PeerSentTooManyPeers.into());
        }

        // Tell the sync mgr about the new incoming core sync data.
        self.core_sync_svc
            .ready()
            .await?
            .call(CoreSyncDataRequest::NewIncoming(peer_core_sync.clone()))
            .await?;

        // Tell the address book about the new peers
        self.address_book
            .ready()
            .await?
            .call(AddressBookRequest::HandleNewPeerList(
                local_peerlist_new,
                self.addr.get_zone(),
            ))
            .await?;

        // This won't actually happen (as long as we have a none 0 minimum support flags)
        // it's just included here for completeness.
        if peer_node_data.support_flags.is_empty() {
            self.send_support_flag_req().await?;
            self.state =
                HandshakeState::WaitingForSupportFlagResponse(peer_node_data, peer_core_sync);
        } else {
            // this will always happen.
            self.state = HandshakeState::Complete(peer_node_data, peer_core_sync);
        }

        Ok(())
    }

    /// Handles a [`MessageResponse`].
    async fn handle_message_response(&mut self, response: MessageResponse) -> Result<(), BoxError> {
        // The functions called here will change the state of the HandshakeSM so `HandshakeState::Start`
        // is just used as a place holder.
        //
        // doing this allows us to not clone the BasicNodeData and CoreSyncData for WaitingForSupportFlagResponse.
        let prv_state = std::mem::replace(&mut self.state, HandshakeState::Start);

        match (prv_state, response) {
            (
                HandshakeState::WaitingForHandshakeResponse,
                MessageResponse::Handshake(handshake),
            ) => self.handle_handshake_response(handshake).await,
            (
                HandshakeState::WaitingForSupportFlagResponse(mut bnd, coresync),
                MessageResponse::SupportFlags(support_flags),
            ) => {
                bnd.support_flags = support_flags.support_flags;
                self.state = HandshakeState::Complete(bnd, coresync);
                Ok(())
            }
            _ => Err(HandShakeError::PeerSentWrongResponse.into()),
        }
    }

    /// Sends our [`PeerSupportFlags`] to the peer.
    async fn send_support_flags(
        &mut self,
        support_flags: PeerSupportFlags,
    ) -> Result<(), HandShakeError> {
        let message = Message::Response(SupportFlagsResponse { support_flags }.into());
        self.peer_sink.send(message).await?;
        Ok(())
    }

    /// Attempts an outbound handshake with the peer.
    async fn do_outbound_handshake(&mut self) -> Result<(), BoxError> {
        // Get the data needed for the handshake request.
        let core_sync = self.get_our_core_sync().await?;
        // send the handshake request.
        self.send_handshake_req(self.basic_node_data.clone(), core_sync)
            .await?;
        // set the state to waiting for a response.
        self.state = HandshakeState::WaitingForHandshakeResponse;

        while !self.state.is_complete() {
            match self.peer_stream.next().await {
                Some(mes) => {
                    let mes = mes?;
                    match mes {
                        Message::Request(MessageRequest::SupportFlags(_)) => {
                            // The only request we should be getting during an outbound handshake
                            // is a support flag request.
                            self.send_support_flags(self.basic_node_data.support_flags)
                                .await?
                        }
                        Message::Response(response) => {
                            // This could be a handshake response or a support flags response.
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

    /// Completes a handshake with a peer.
    async fn do_handshake(mut self) -> Result<Client, BoxError> {
        let mut peer_reachable = false;
        match self.addr.direction() {
            Direction::Outbound => {
                self.do_outbound_handshake().await?;
                // If this is an outbound handshake the obviously the peer
                // is reachable.
                peer_reachable = true
            }
            Direction::Inbound => todo!(),
        }

        let (server_tx, server_rx) = mpsc::channel(0);

        let (peer_node_data, coresync) = self
            .state
            .peer_data()
            .expect("We must be in state complete to be here");

        let pruning_seed = PruningSeed::try_from(coresync.pruning_seed).map_err(|e| Box::new(e))?;

        // create the handle between the Address book and the connection task to
        // allow the address book to shutdown the connection task and to update
        // the address book when the connection is closed.
        let (book_connection_side_handle, connection_book_side_handle) =
            new_address_book_connection_handle();

        // tell the address book about the new connection.
        self.address_book
            .ready()
            .await?
            .call(AddressBookRequest::ConnectedToPeer {
                zone: self.addr.get_zone(),
                connection_handle: connection_book_side_handle,
                addr: self.addr.get_network_address(
                    peer_node_data
                        .my_port
                        .try_into()
                        .map_err(|_| "Peer sent a port that does not fit into a u16")?,
                ),
                id: peer_node_data.peer_id,
                reachable: peer_reachable,
                last_seen: chrono::Utc::now().naive_utc(),
                pruning_seed,
                rpc_port: peer_node_data.rpc_port,
                rpc_credits_per_hash: peer_node_data.rpc_credits_per_hash,
            })
            .await?;

        // This block below is for keeping the last seen times in the address book
        // upto date. We only update the last seen times on timed syncs to reduce
        // the load on the address book.
        //
        // first clone the items needed
        let mut address_book = self.address_book.clone();
        let peer_id = peer_node_data.peer_id;
        let net_zone = self.addr.get_zone();
        let peer_stream = self.peer_stream.then(|mes| async move {
            if let Ok(mes) = &mes {
                if mes.id() == TimedSync::ID {
                    if let Ok(ready_book) = address_book.ready().await {
                        // we dont care about address book errors here, If there is a problem
                        // with the address book the node will get shutdown.
                        let _ = ready_book
                            .call(AddressBookRequest::SetPeerSeen(
                                peer_id,
                                chrono::Utc::now().naive_utc(),
                                net_zone,
                            ))
                            .await;
                    }
                }
            }
            // return the message
            mes
        });

        let connection = Connection::new(
            self.addr,
            self.peer_sink,
            server_rx,
            self.connection_tracker,
            book_connection_side_handle,
            self.peer_request_service,
        );

        let connection_task = tokio::task::spawn(connection.run().instrument(self.connection_span));

        let connection_info = ConnectionInfo {
            addr: self.addr,
            support_flags: peer_node_data.support_flags,
            pruning_seed,
            peer_id: peer_node_data.peer_id,
            rpc_port: peer_node_data.rpc_port,
            rpc_credits_per_hash: peer_node_data.rpc_credits_per_hash,
        };

        let client = Client::new(connection_info.into(), server_tx, connection_task);

        Ok(client)
    }
}

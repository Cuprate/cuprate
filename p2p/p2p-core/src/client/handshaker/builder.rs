use std::{
    future::{ready, Ready},
    marker::PhantomData,
    task::{Context, Poll},
};

use futures::{stream, Stream};
use tower::Service;
use tracing::Span;

use cuprate_wire::{BasicNodeData, CoreSyncData};

use crate::{
    client::{handshaker::HandShaker, InternalPeerID},
    services::{
        AddressBookRequest, AddressBookResponse, CoreSyncDataRequest, CoreSyncDataResponse,
        PeerSyncRequest, PeerSyncResponse,
    },
    AddressBook, BroadcastMessage, CoreSyncSvc, NetworkZone, PeerRequest, PeerRequestHandler,
    PeerResponse, PeerSyncSvc, ProtocolResponse,
};

/// A [`HandShaker`] [`Service`] builder.
///
/// This builder applies default values to make usage easier, behaviour and drawbacks of the defaults are documented
/// on the `with_*` method to change it, for example [`HandshakerBuilder::with_peer_request_handler`].
///
/// If you want to use any network other than mainnet you will need to change the core sync service with
/// [`HandshakerBuilder::with_core_sync_svc`], see that method for details.
#[derive(Debug, Clone)]
pub struct HandshakerBuilder<
    N: NetworkZone,
    AdrBook = DummyAddressBook,
    CSync = DummyCoreSyncSvc,
    PSync = DummyPeerSyncSvc,
    ReqHdlr = DummyPeerRequestHdlr,
    BrdcstStrmMkr = fn(
        InternalPeerID<<N as NetworkZone>::Addr>,
    ) -> stream::Pending<BroadcastMessage>,
> {
    /// The address book service.
    address_book: AdrBook,
    /// The core sync data service.
    core_sync_svc: CSync,
    /// The peer sync service.
    peer_sync_svc: PSync,
    /// The peer request handler service.
    peer_request_svc: ReqHdlr,

    /// Our [`BasicNodeData`]
    our_basic_node_data: BasicNodeData,

    /// A function that returns a stream that will give items to be broadcast by a connection.
    broadcast_stream_maker: BrdcstStrmMkr,

    connection_parent_span: Option<Span>,

    /// The network zone.
    _zone: PhantomData<N>,
}

impl<N: NetworkZone> HandshakerBuilder<N> {
    /// Creates a new builder with our nodes basic node data.
    pub fn new(our_basic_node_data: BasicNodeData) -> Self {
        Self {
            address_book: DummyAddressBook,
            core_sync_svc: DummyCoreSyncSvc::static_mainnet_genesis(),
            peer_sync_svc: DummyPeerSyncSvc,
            peer_request_svc: DummyPeerRequestHdlr,
            our_basic_node_data,
            broadcast_stream_maker: |_| stream::pending(),
            connection_parent_span: None,
            _zone: PhantomData,
        }
    }
}

impl<N: NetworkZone, AdrBook, CSync, PSync, ReqHdlr, BrdcstStrmMkr>
    HandshakerBuilder<N, AdrBook, CSync, PSync, ReqHdlr, BrdcstStrmMkr>
{
    /// Changes the address book to the provided one.
    ///
    /// ## Default Address Book
    ///
    /// The default address book is used if this function is not called.
    ///
    /// The default address book's only drawback is that it does not keep track of peers. Which means
    /// connections should not be terminated early.
    pub fn with_address_book<NAdrBook>(
        self,
        new_address_book: NAdrBook,
    ) -> HandshakerBuilder<N, NAdrBook, CSync, PSync, ReqHdlr, BrdcstStrmMkr>
    where
        NAdrBook: AddressBook<N> + Clone,
    {
        let HandshakerBuilder {
            core_sync_svc,
            peer_sync_svc,
            peer_request_svc,
            our_basic_node_data,
            broadcast_stream_maker,
            connection_parent_span,
            _zone,
            ..
        } = self;

        HandshakerBuilder {
            address_book: new_address_book,
            core_sync_svc,
            peer_sync_svc,
            peer_request_svc,
            our_basic_node_data,
            broadcast_stream_maker,
            connection_parent_span,
            _zone,
        }
    }

    /// Changes the core sync service to the provided one.
    ///
    /// The core sync service should keep track of our nodes core sync data.
    ///
    /// ## Default Core Sync Service
    ///
    /// The default core sync service is used if this method is not called.
    ///
    /// The default core sync service will just use the mainnet genesis block, to use other network's
    /// genesis see [`DummyCoreSyncSvc::static_stagenet_genesis`] and [`DummyCoreSyncSvc::static_testnet_genesis`].
    /// The drawbacks to keeping this the default is that it will always return the mainnet genesis as our nodes
    /// sync info, which means peers won't know our actual chain height, this may or may not be a problem for
    /// different use cases.
    pub fn with_core_sync_svc<NCSync>(
        self,
        new_core_sync_svc: NCSync,
    ) -> HandshakerBuilder<N, AdrBook, NCSync, PSync, ReqHdlr, BrdcstStrmMkr>
    where
        NCSync: CoreSyncSvc + Clone,
    {
        let HandshakerBuilder {
            address_book,
            peer_sync_svc,
            peer_request_svc,
            our_basic_node_data,
            broadcast_stream_maker,
            connection_parent_span,
            _zone,
            ..
        } = self;

        HandshakerBuilder {
            address_book,
            core_sync_svc: new_core_sync_svc,
            peer_sync_svc,
            peer_request_svc,
            our_basic_node_data,
            broadcast_stream_maker,
            connection_parent_span,
            _zone,
        }
    }

    /// Changes the peer sync service, which keeps track of peers sync states.
    ///
    /// ## Default Peer Sync Service
    ///
    /// The default peer sync service will be used if this method is not called.
    ///
    /// The default peer sync service will not keep track of peers sync states.
    pub fn with_peer_sync_svc<NPSync>(
        self,
        new_peer_sync_svc: NPSync,
    ) -> HandshakerBuilder<N, AdrBook, CSync, NPSync, ReqHdlr, BrdcstStrmMkr>
    where
        NPSync: PeerSyncSvc<N> + Clone,
    {
        let HandshakerBuilder {
            address_book,
            core_sync_svc,
            peer_request_svc,
            our_basic_node_data,
            broadcast_stream_maker,
            connection_parent_span,
            _zone,
            ..
        } = self;

        HandshakerBuilder {
            address_book,
            core_sync_svc,
            peer_sync_svc: new_peer_sync_svc,
            peer_request_svc,
            our_basic_node_data,
            broadcast_stream_maker,
            connection_parent_span,
            _zone,
        }
    }

    /// Changes the peer request handler.
    ///
    /// ## Default Peer Request Handler
    ///
    /// The default peer request handler will be used if this method is not called.
    ///
    /// The default request handler does not respond to requests, which means connections will probably be
    /// dropped within a couple of minutes after handshaking. This will be alright for some purposes, but
    /// you will need to if you want to hold connections for longer than a few minutes.
    pub fn with_peer_request_handler<NReqHdlr>(
        self,
        new_peer_request_svc: NReqHdlr,
    ) -> HandshakerBuilder<N, AdrBook, CSync, PSync, NReqHdlr, BrdcstStrmMkr>
    where
        NReqHdlr: PeerRequestHandler + Clone,
    {
        let HandshakerBuilder {
            address_book,
            core_sync_svc,
            peer_sync_svc,
            our_basic_node_data,
            broadcast_stream_maker,
            connection_parent_span,
            _zone,
            ..
        } = self;

        HandshakerBuilder {
            address_book,
            core_sync_svc,
            peer_sync_svc,
            peer_request_svc: new_peer_request_svc,
            our_basic_node_data,
            broadcast_stream_maker,
            connection_parent_span,
            _zone,
        }
    }

    /// Changes the broadcast stream maker, which is used to create streams that yield messages to broadcast.
    ///
    /// ## Default Broadcast Stream Maker
    ///
    /// The default broadcast stream maker just returns [`stream::Pending`], i.e. the returned stream will not
    /// produce any messages to broadcast, this is not a problem if your use case does not require broadcasting
    /// message.
    pub fn with_broadcast_stream_maker<NBrdcstStrmMkr, BrdcstStrm>(
        self,
        new_broadcast_stream_maker: NBrdcstStrmMkr,
    ) -> HandshakerBuilder<N, AdrBook, CSync, PSync, ReqHdlr, NBrdcstStrmMkr>
    where
        BrdcstStrm: Stream<Item = BroadcastMessage> + Send + 'static,
        NBrdcstStrmMkr: Fn(InternalPeerID<N::Addr>) -> BrdcstStrm + Clone + Send + 'static,
    {
        let HandshakerBuilder {
            address_book,
            core_sync_svc,
            peer_sync_svc,
            peer_request_svc,
            our_basic_node_data,
            connection_parent_span,
            _zone,
            ..
        } = self;

        HandshakerBuilder {
            address_book,
            core_sync_svc,
            peer_sync_svc,
            peer_request_svc,
            our_basic_node_data,
            broadcast_stream_maker: new_broadcast_stream_maker,
            connection_parent_span,
            _zone,
        }
    }

    /// Changes the parent [`Span`] of the connection task to the one provided.
    ///
    /// ## Default Broadcast Stream Maker
    ///
    /// The default connection span will be [`Span::none`].
    pub fn with_connection_parent_span(self, connection_parent_span: Span) -> Self {
        Self {
            connection_parent_span: Some(connection_parent_span),
            ..self
        }
    }

    /// Builds the [`HandShaker`].
    pub fn build(self) -> HandShaker<N, AdrBook, CSync, PSync, ReqHdlr, BrdcstStrmMkr> {
        HandShaker::new(
            self.address_book,
            self.peer_sync_svc,
            self.core_sync_svc,
            self.peer_request_svc,
            self.broadcast_stream_maker,
            self.our_basic_node_data,
            self.connection_parent_span.unwrap_or(Span::none()),
        )
    }
}

/// A dummy peer request handler, that doesn't respond to any requests.
#[derive(Debug, Clone)]
pub struct DummyPeerRequestHdlr;

impl Service<PeerRequest> for DummyPeerRequestHdlr {
    type Response = PeerResponse;
    type Error = tower::BoxError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: PeerRequest) -> Self::Future {
        ready(Ok(PeerResponse::Protocol(ProtocolResponse::NA)))
    }
}

/// A dummy peer sync service, that doesn't actually keep track of peers sync states.
#[derive(Debug, Clone)]
pub struct DummyPeerSyncSvc;

impl<N: NetworkZone> Service<PeerSyncRequest<N>> for DummyPeerSyncSvc {
    type Response = PeerSyncResponse<N>;
    type Error = tower::BoxError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: PeerSyncRequest<N>) -> Self::Future {
        ready(Ok(match req {
            PeerSyncRequest::PeersToSyncFrom { .. } => PeerSyncResponse::PeersToSyncFrom(vec![]),
            PeerSyncRequest::IncomingCoreSyncData(_, _, _) => PeerSyncResponse::Ok,
        }))
    }
}

/// A dummy core sync service that just returns static [`CoreSyncData`].
#[derive(Debug, Clone)]
pub struct DummyCoreSyncSvc(CoreSyncData);

impl DummyCoreSyncSvc {
    /// Returns a [`DummyCoreSyncSvc`] that will just return the mainnet genesis [`CoreSyncData`].
    pub fn static_mainnet_genesis() -> DummyCoreSyncSvc {
        DummyCoreSyncSvc(CoreSyncData {
            cumulative_difficulty: 1,
            cumulative_difficulty_top64: 0,
            current_height: 1,
            pruning_seed: 0,
            top_id: hex_literal::hex!(
                "418015bb9ae982a1975da7d79277c2705727a56894ba0fb246adaabb1f4632e3"
            ),
            top_version: 1,
        })
    }

    /// Returns a [`DummyCoreSyncSvc`] that will just return the testnet genesis [`CoreSyncData`].
    pub fn static_testnet_genesis() -> DummyCoreSyncSvc {
        DummyCoreSyncSvc(CoreSyncData {
            cumulative_difficulty: 1,
            cumulative_difficulty_top64: 0,
            current_height: 1,
            pruning_seed: 0,
            top_id: hex_literal::hex!(
                "48ca7cd3c8de5b6a4d53d2861fbdaedca141553559f9be9520068053cda8430b"
            ),
            top_version: 1,
        })
    }

    /// Returns a [`DummyCoreSyncSvc`] that will just return the stagenet genesis [`CoreSyncData`].
    pub fn static_stagenet_genesis() -> DummyCoreSyncSvc {
        DummyCoreSyncSvc(CoreSyncData {
            cumulative_difficulty: 1,
            cumulative_difficulty_top64: 0,
            current_height: 1,
            pruning_seed: 0,
            top_id: hex_literal::hex!(
                "76ee3cc98646292206cd3e86f74d88b4dcc1d937088645e9b0cbca84b7ce74eb"
            ),
            top_version: 1,
        })
    }

    /// Returns a [`DummyCoreSyncSvc`] that will return the provided [`CoreSyncData`].
    pub fn static_custom(data: CoreSyncData) -> DummyCoreSyncSvc {
        DummyCoreSyncSvc(data)
    }
}

impl Service<CoreSyncDataRequest> for DummyCoreSyncSvc {
    type Response = CoreSyncDataResponse;
    type Error = tower::BoxError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: CoreSyncDataRequest) -> Self::Future {
        ready(Ok(CoreSyncDataResponse(self.0.clone())))
    }
}

/// A dummy address book that doesn't actually keep track of peers.
#[derive(Debug, Clone)]
pub struct DummyAddressBook;

impl<N: NetworkZone> Service<AddressBookRequest<N>> for DummyAddressBook {
    type Response = AddressBookResponse<N>;
    type Error = tower::BoxError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: AddressBookRequest<N>) -> Self::Future {
        ready(Ok(match req {
            AddressBookRequest::GetWhitePeers(_) => AddressBookResponse::Peers(vec![]),
            AddressBookRequest::TakeRandomGrayPeer { .. }
            | AddressBookRequest::TakeRandomPeer { .. }
            | AddressBookRequest::TakeRandomWhitePeer { .. } => {
                return ready(Err("dummy address book does not hold peers".into()));
            }
            AddressBookRequest::NewConnection { .. } | AddressBookRequest::IncomingPeerList(_) => {
                AddressBookResponse::Ok
            }
            AddressBookRequest::IsPeerBanned(_) => AddressBookResponse::IsPeerBanned(false),
        }))
    }
}

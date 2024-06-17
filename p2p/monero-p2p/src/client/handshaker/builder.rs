use futures::{stream, Stream};
use std::future::{ready, Ready};
use std::marker::PhantomData;
use std::task::{Context, Poll};
use tower::Service;
use tracing::Span;

use monero_wire::{BasicNodeData, CoreSyncData};

use crate::client::{handshaker::HandShaker, InternalPeerID};
use crate::services::{
    AddressBookRequest, AddressBookResponse, CoreSyncDataRequest, CoreSyncDataResponse,
    PeerSyncRequest, PeerSyncResponse,
};
use crate::{
    AddressBook, BroadcastMessage, CoreSyncSvc, NetworkZone, PeerRequest, PeerRequestHandler,
    PeerResponse, PeerSyncSvc,
};

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

    pub fn with_connection_parent_span(self, connection_parent_span: Span) -> Self {
        Self {
            connection_parent_span: Some(connection_parent_span),
            ..self
        }
    }

    pub fn build(self) -> HandShaker<N, AdrBook, CSync, PSync, ReqHdlr, BrdcstStrmMkr> {
        HandShaker::new(
            self.address_book,
            self.peer_sync_svc,
            self.core_sync_svc,
            self.peer_request_svc,
            self.broadcast_stream_maker,
            self.our_basic_node_data,
            self.connection_parent_span.unwrap_or(Span::current()),
        )
    }
}

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
        ready(Ok(PeerResponse::NA))
    }
}

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

#[derive(Debug, Clone)]
pub struct DummyCoreSyncSvc(CoreSyncData);

impl DummyCoreSyncSvc {
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

#[derive(Debug, Clone)]
pub struct DummyAddressBook;

impl<N: NetworkZone> Service<AddressBookRequest<N>> for DummyAddressBook {
    type Response = AddressBookResponse<N>;
    type Error = tower::BoxError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Pending
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

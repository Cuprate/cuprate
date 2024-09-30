use std::marker::PhantomData;

use futures::{stream, Stream};
use tracing::Span;

use cuprate_wire::BasicNodeData;

use crate::{
    client::{handshaker::HandShaker, InternalPeerID},
    AddressBook, BroadcastMessage, CoreSyncSvc, NetworkZone, PeerSyncSvc,
    ProtocolRequestHandlerMaker,
};

mod dummy;
pub use dummy::{
    DummyAddressBook, DummyCoreSyncSvc, DummyPeerSyncSvc, DummyProtocolRequestHandler,
    DummyProtocolRequestHandlerMaker,
};

/// A [`HandShaker`] [`Service`](tower::Service) builder.
///
/// This builder applies default values to make usage easier, behaviour and drawbacks of the defaults are documented
/// on the `with_*` method to change it, for example [`HandshakerBuilder::with_protocol_request_handler`].
///
/// If you want to use any network other than [`Mainnet`](crate::Network::Mainnet)
/// you will need to change the core sync service with [`HandshakerBuilder::with_core_sync_svc`],
/// see that method for details.
#[derive(Debug, Clone)]
pub struct HandshakerBuilder<
    N: NetworkZone,
    AdrBook = DummyAddressBook,
    CSync = DummyCoreSyncSvc,
    PSync = DummyPeerSyncSvc,
    ProtoHdlrMkr = DummyProtocolRequestHandlerMaker,
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
    /// The protocol request service.
    protocol_request_svc_maker: ProtoHdlrMkr,
    /// Our [`BasicNodeData`]
    our_basic_node_data: BasicNodeData,
    /// A function that returns a stream that will give items to be broadcast by a connection.
    broadcast_stream_maker: BrdcstStrmMkr,
    /// The [`Span`] that will set as the parent to the connection [`Span`].
    connection_parent_span: Option<Span>,

    /// The network zone.
    _zone: PhantomData<N>,
}

impl<N: NetworkZone> HandshakerBuilder<N> {
    /// Creates a new builder with our node's basic node data.
    pub fn new(our_basic_node_data: BasicNodeData) -> Self {
        Self {
            address_book: DummyAddressBook,
            core_sync_svc: DummyCoreSyncSvc::static_mainnet_genesis(),
            peer_sync_svc: DummyPeerSyncSvc,
            protocol_request_svc_maker: DummyProtocolRequestHandlerMaker,
            our_basic_node_data,
            broadcast_stream_maker: |_| stream::pending(),
            connection_parent_span: None,
            _zone: PhantomData,
        }
    }
}

impl<N: NetworkZone, AdrBook, CSync, PSync, ProtoHdlr, BrdcstStrmMkr>
    HandshakerBuilder<N, AdrBook, CSync, PSync, ProtoHdlr, BrdcstStrmMkr>
{
    /// Changes the address book to the provided one.
    ///
    /// ## Default Address Book
    ///
    /// The default address book is used if this function is not called.
    ///
    /// The default address book's only drawback is that it does not keep track of peers and therefore
    /// bans.
    pub fn with_address_book<NAdrBook>(
        self,
        new_address_book: NAdrBook,
    ) -> HandshakerBuilder<N, NAdrBook, CSync, PSync, ProtoHdlr, BrdcstStrmMkr>
    where
        NAdrBook: AddressBook<N> + Clone,
    {
        let Self {
            core_sync_svc,
            peer_sync_svc,
            protocol_request_svc_maker,
            our_basic_node_data,
            broadcast_stream_maker,
            connection_parent_span,
            ..
        } = self;

        HandshakerBuilder {
            address_book: new_address_book,
            core_sync_svc,
            peer_sync_svc,
            protocol_request_svc_maker,
            our_basic_node_data,
            broadcast_stream_maker,
            connection_parent_span,
            _zone: PhantomData,
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
    ) -> HandshakerBuilder<N, AdrBook, NCSync, PSync, ProtoHdlr, BrdcstStrmMkr>
    where
        NCSync: CoreSyncSvc + Clone,
    {
        let Self {
            address_book,
            peer_sync_svc,
            protocol_request_svc_maker,
            our_basic_node_data,
            broadcast_stream_maker,
            connection_parent_span,
            ..
        } = self;

        HandshakerBuilder {
            address_book,
            core_sync_svc: new_core_sync_svc,
            peer_sync_svc,
            protocol_request_svc_maker,
            our_basic_node_data,
            broadcast_stream_maker,
            connection_parent_span,
            _zone: PhantomData,
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
    ) -> HandshakerBuilder<N, AdrBook, CSync, NPSync, ProtoHdlr, BrdcstStrmMkr>
    where
        NPSync: PeerSyncSvc<N> + Clone,
    {
        let Self {
            address_book,
            core_sync_svc,
            protocol_request_svc_maker,
            our_basic_node_data,
            broadcast_stream_maker,
            connection_parent_span,
            ..
        } = self;

        HandshakerBuilder {
            address_book,
            core_sync_svc,
            peer_sync_svc: new_peer_sync_svc,
            protocol_request_svc_maker,
            our_basic_node_data,
            broadcast_stream_maker,
            connection_parent_span,
            _zone: PhantomData,
        }
    }

    /// Changes the protocol request handler maker, which creates the service that handles [`ProtocolRequest`](crate::ProtocolRequest)s
    /// to our node.
    ///
    /// ## Default Protocol Request Handler
    ///
    /// The default service maker will create services that will not respond to any protocol requests, this should not
    /// be an issue as long as peers do not think we are ahead of them, if they do they will send requests
    /// for our blocks, and we won't respond which will cause them to disconnect.
    pub fn with_protocol_request_handler_maker<NProtoHdlrMkr>(
        self,
        new_protocol_request_svc_maker: NProtoHdlrMkr,
    ) -> HandshakerBuilder<N, AdrBook, CSync, PSync, NProtoHdlrMkr, BrdcstStrmMkr>
    where
        NProtoHdlrMkr: ProtocolRequestHandlerMaker<N> + Clone,
    {
        let Self {
            address_book,
            core_sync_svc,
            peer_sync_svc,
            our_basic_node_data,
            broadcast_stream_maker,
            connection_parent_span,
            ..
        } = self;

        HandshakerBuilder {
            address_book,
            core_sync_svc,
            peer_sync_svc,
            protocol_request_svc_maker: new_protocol_request_svc_maker,
            our_basic_node_data,
            broadcast_stream_maker,
            connection_parent_span,
            _zone: PhantomData,
        }
    }

    /// Changes the broadcast stream maker, which is used to create streams that yield messages to broadcast.
    ///
    /// ## Default Broadcast Stream Maker
    ///
    /// The default broadcast stream maker just returns [`stream::Pending`], i.e. the returned stream will not
    /// produce any messages to broadcast, this is not a problem if your use case does not require broadcasting
    /// messages.
    pub fn with_broadcast_stream_maker<NBrdcstStrmMkr, BrdcstStrm>(
        self,
        new_broadcast_stream_maker: NBrdcstStrmMkr,
    ) -> HandshakerBuilder<N, AdrBook, CSync, PSync, ProtoHdlr, NBrdcstStrmMkr>
    where
        BrdcstStrm: Stream<Item = BroadcastMessage> + Send + 'static,
        NBrdcstStrmMkr: Fn(InternalPeerID<N::Addr>) -> BrdcstStrm + Clone + Send + 'static,
    {
        let Self {
            address_book,
            core_sync_svc,
            peer_sync_svc,
            protocol_request_svc_maker,
            our_basic_node_data,
            connection_parent_span,
            ..
        } = self;

        HandshakerBuilder {
            address_book,
            core_sync_svc,
            peer_sync_svc,
            protocol_request_svc_maker,
            our_basic_node_data,
            broadcast_stream_maker: new_broadcast_stream_maker,
            connection_parent_span,
            _zone: PhantomData,
        }
    }

    /// Changes the parent [`Span`] of the connection task to the one provided.
    ///
    /// ## Default Connection Parent Span
    ///
    /// The default connection span will be [`Span::none`].
    #[must_use]
    pub fn with_connection_parent_span(self, connection_parent_span: Span) -> Self {
        Self {
            connection_parent_span: Some(connection_parent_span),
            ..self
        }
    }

    /// Builds the [`HandShaker`].
    pub fn build(self) -> HandShaker<N, AdrBook, CSync, PSync, ProtoHdlr, BrdcstStrmMkr> {
        HandShaker::new(
            self.address_book,
            self.peer_sync_svc,
            self.core_sync_svc,
            self.protocol_request_svc_maker,
            self.broadcast_stream_maker,
            self.our_basic_node_data,
            self.connection_parent_span.unwrap_or(Span::none()),
        )
    }
}

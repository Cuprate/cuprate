//! Cuprate's P2P Crate.
//!
//! This crate contains a [`NetworkInterface`] which allows interacting with the Monero P2P network on
//! a certain [`NetworkZone`]
use std::sync::Arc;

use futures::FutureExt;
use tokio::{
    sync::{mpsc, watch},
    task::JoinSet,
};
use tokio_stream::wrappers::WatchStream;
use tower::{buffer::Buffer, util::BoxCloneService, ServiceExt};
use tracing::{instrument, Instrument, Span};

use monero_p2p::{
    client::Connector,
    client::InternalPeerID,
    services::{AddressBookRequest, AddressBookResponse},
    CoreSyncSvc, NetworkZone, PeerRequestHandler,
};

mod broadcast;
mod client_pool;
pub mod config;
pub mod connection_maintainer;
mod constants;
mod inbound_server;
mod sync_states;

pub use broadcast::{BroadcastRequest, BroadcastSvc};
use client_pool::ClientPoolDropGuard;
pub use config::P2PConfig;
use connection_maintainer::MakeConnectionRequest;

/// Initializes the P2P [`NetworkInterface`] for a specific [`NetworkZone`].
///
/// This function starts all the tasks to maintain/accept/make connections.
///
/// # Usage
/// You must provide:
/// - A peer request handler, which is given to each connection
/// - A core sync service, which keeps track of the sync state of our node
#[instrument(level = "debug", name = "net", skip_all, fields(zone = N::NAME))]
pub async fn initialize_network<N, R, CS>(
    peer_req_handler: R,
    core_sync_svc: CS,
    config: P2PConfig<N>,
) -> Result<NetworkInterface<N>, tower::BoxError>
where
    N: NetworkZone,
    R: PeerRequestHandler + Clone,
    CS: CoreSyncSvc + Clone,
{
    let address_book =
        monero_address_book::init_address_book(config.address_book_config.clone()).await?;
    let address_book = Buffer::new(
        address_book,
        config.max_inbound_connections + config.outbound_connections,
    );

    let (sync_states_svc, top_block_watch) = sync_states::PeerSyncSvc::new();
    let sync_states_svc = Buffer::new(
        sync_states_svc,
        config.max_inbound_connections + config.outbound_connections,
    );

    // Use the default config. Changing the defaults affects tx fluff times, which could affect D++ so for now don't allow changing
    // this.
    let (broadcast_svc, outbound_mkr, inbound_mkr) =
        broadcast::init_broadcast_channels(broadcast::BroadcastConfig::default());

    let mut basic_node_data = config.basic_node_data();

    if !N::CHECK_NODE_ID {
        basic_node_data.peer_id = 1;
    }

    let outbound_handshaker_builder = monero_p2p::client::HandshakerBuilder::new(basic_node_data)
        .with_address_book(address_book.clone())
        .with_peer_sync_svc(sync_states_svc)
        .with_core_sync_svc(core_sync_svc)
        .with_peer_request_handler(peer_req_handler)
        .with_broadcast_stream_maker(outbound_mkr)
        .with_connection_parent_span(Span::current());

    let inbound_handshaker = outbound_handshaker_builder
        .clone()
        .with_broadcast_stream_maker(inbound_mkr)
        .build();

    let outbound_handshaker = outbound_handshaker_builder.build();

    let client_pool = client_pool::ClientPool::new();

    let (make_connection_tx, make_connection_rx) = mpsc::channel(3);

    let outbound_connector = Connector::new(outbound_handshaker);
    let outbound_connection_maintainer = connection_maintainer::OutboundConnectionKeeper::new(
        config.clone(),
        client_pool.clone(),
        make_connection_rx,
        address_book.clone(),
        outbound_connector,
    );

    let mut background_tasks = JoinSet::new();

    background_tasks.spawn(
        outbound_connection_maintainer
            .run()
            .instrument(Span::current()),
    );
    background_tasks.spawn(
        inbound_server::inbound_server(
            client_pool.clone(),
            inbound_handshaker,
            address_book.clone(),
            config,
        )
        .map(|res| {
            if let Err(e) = res {
                tracing::error!("Error in inbound connection listener: {e}")
            }

            tracing::info!("Inbound connection listener shutdown")
        })
        .instrument(Span::current()),
    );

    Ok(NetworkInterface {
        pool: client_pool,
        broadcast_svc,
        top_block_watch,
        make_connection_tx,
        address_book: address_book.boxed_clone(),
        _background_tasks: Arc::new(background_tasks),
    })
}

/// The interface to Monero's P2P network on a certain [`NetworkZone`].
#[derive(Clone)]
pub struct NetworkInterface<N: NetworkZone> {
    /// A pool of free connected peers.
    pool: Arc<client_pool::ClientPool<N>>,
    /// A [`Service`] that allows broadcasting to all connected peers.
    broadcast_svc: BroadcastSvc<N>,
    /// A [`watch`] channel that contains the highest seen cumulative difficulty and other info
    /// on that claimed chain.
    top_block_watch: watch::Receiver<sync_states::NewSyncInfo>,
    /// A channel to request extra connections.
    #[allow(dead_code)] // will be used eventually
    make_connection_tx: mpsc::Sender<MakeConnectionRequest>,
    /// The address book service.
    address_book: BoxCloneService<AddressBookRequest<N>, AddressBookResponse<N>, tower::BoxError>,
    /// Background tasks that will be aborted when this interface is dropped.
    _background_tasks: Arc<JoinSet<()>>,
}

impl<N: NetworkZone> NetworkInterface<N> {
    /// Returns a service which allows broadcasting messages to all the connected peers in a specific [`NetworkZone`].
    pub fn broadcast_svc(&self) -> BroadcastSvc<N> {
        self.broadcast_svc.clone()
    }

    /// Returns a stream which yields the highest seen sync state from a connected peer.
    pub fn top_sync_stream(&self) -> WatchStream<sync_states::NewSyncInfo> {
        WatchStream::from_changes(self.top_block_watch.clone())
    }

    /// Returns the address book service.
    pub fn address_book(
        &self,
    ) -> BoxCloneService<AddressBookRequest<N>, AddressBookResponse<N>, tower::BoxError> {
        self.address_book.clone()
    }

    /// Pulls a client from the client pool, returning it in a guard that will return it there when it's
    /// dropped.
    pub fn borrow_client(&self, peer: &InternalPeerID<N::Addr>) -> Option<ClientPoolDropGuard<N>> {
        self.pool.borrow_client(peer)
    }
}

//! Cuprate's P2P Crate.
//!
//! This crate contains a [`NetworkInterface`] which allows interacting with the Monero P2P network on
//! a certain [`NetworkZone`]
use std::sync::Arc;

use futures::FutureExt;
use tokio::{
    sync::{mpsc, Notify},
    task::JoinSet,
    time::{sleep, Duration},
};
use tower::{buffer::Buffer, util::BoxCloneService, Service, ServiceExt};
use tracing::{instrument, Instrument, Span};

use cuprate_async_buffer::BufferStream;
use cuprate_p2p_core::{
    client::Connector,
    services::{AddressBookRequest, AddressBookResponse},
    CoreSyncSvc, NetworkZone, ProtocolRequestHandlerMaker, Transport,
};

pub mod block_downloader;
mod broadcast;
pub mod config;
pub mod connection_maintainer;
pub mod constants;
mod inbound_server;
mod peer_set;

use block_downloader::{BlockBatch, BlockDownloaderConfig, ChainSvcRequest, ChainSvcResponse};
pub use broadcast::{BroadcastRequest, BroadcastSvc};
pub use config::{AddressBookConfig, P2PConfig, TransportConfig};
use connection_maintainer::MakeConnectionRequest;
use peer_set::PeerSet;
pub use peer_set::{ClientDropGuard, PeerSetRequest, PeerSetResponse};

/// Interval for checking inbound connection status (1 hour)
const INBOUND_CONNECTION_MONITOR_INTERVAL: Duration = Duration::from_secs(3600);

/// Monitors for inbound connections and logs a warning if none are detected.
///
/// This task runs every hour to check if there are inbound connections available.
/// If `max_inbound_connections` is 0, the task will exit without logging.
#[expect(clippy::infinite_loop)]
async fn inbound_connection_monitor(
    inbound_semaphore: Arc<tokio::sync::Semaphore>,
    max_inbound_connections: usize,
    p2p_port: u16,
) {
    // Skip monitoring if inbound connections are disabled
    if max_inbound_connections == 0 {
        return;
    }

    loop {
        // Wait for the monitoring interval
        sleep(INBOUND_CONNECTION_MONITOR_INTERVAL).await;

        // Check if we have any inbound connections
        // If available permits equals max_inbound_connections, no peers are connected
        let available_permits = inbound_semaphore.available_permits();
        if available_permits == max_inbound_connections {
            tracing::warn!(
                "No incoming connections - check firewalls/routers allow port {}",
                p2p_port
            );
        }
    }
}

/// Initializes the P2P [`NetworkInterface`] for a specific [`NetworkZone`].
///
/// This function starts all the tasks to maintain/accept/make connections.
///
/// # Usage
/// You must provide:
/// - A protocol request handler, which is given to each connection
/// - A core sync service, which keeps track of the sync state of our node
#[instrument(level = "error", name = "net", skip_all, fields(zone = Z::NAME))]
pub async fn initialize_network<Z, T, PR, CS>(
    protocol_request_handler_maker: PR,
    core_sync_svc: CS,
    config: P2PConfig<Z>,
    transport_config: TransportConfig<Z, T>,
    syncer_wake: Option<Arc<Notify>>,
) -> Result<NetworkInterface<Z>, tower::BoxError>
where
    Z: NetworkZone,
    T: Transport<Z>,
    Z::Addr: borsh::BorshDeserialize + borsh::BorshSerialize,
    PR: ProtocolRequestHandlerMaker<Z> + Clone,
    CS: CoreSyncSvc + Clone,
{
    let address_book =
        cuprate_address_book::init_address_book(config.address_book_config.clone()).await?;
    let address_book = Buffer::new(
        address_book,
        config
            .max_inbound_connections
            .checked_add(config.outbound_connections)
            .unwrap(),
    );

    // Use the default config. Changing the defaults affects tx fluff times, which could affect D++ so for now don't allow changing
    // this.
    let (broadcast_svc, outbound_mkr, inbound_mkr) =
        broadcast::init_broadcast_channels(broadcast::BroadcastConfig::default());

    let mut basic_node_data = config.basic_node_data();

    if !Z::CHECK_NODE_ID {
        basic_node_data.peer_id = 1;
    }

    let mut outbound_handshaker_builder =
        cuprate_p2p_core::client::HandshakerBuilder::<Z, T, _, _, _, _>::new(
            basic_node_data,
            transport_config.client_config,
        )
        .with_address_book(address_book.clone())
        .with_core_sync_svc(core_sync_svc)
        .with_protocol_request_handler_maker(protocol_request_handler_maker)
        .with_broadcast_stream_maker(outbound_mkr)
        .with_connection_parent_span(Span::current());

    if let Some(ref sw) = syncer_wake {
        outbound_handshaker_builder = outbound_handshaker_builder.with_syncer_wake(Arc::clone(sw));
    }

    let inbound_handshaker = outbound_handshaker_builder
        .clone()
        .with_broadcast_stream_maker(inbound_mkr)
        .build();

    let outbound_handshaker = outbound_handshaker_builder.build();

    let (new_connection_tx, new_connection_rx) = mpsc::channel(
        config
            .outbound_connections
            .checked_add(config.max_inbound_connections)
            .unwrap(),
    );
    let (make_connection_tx, make_connection_rx) = mpsc::channel(3);

    let outbound_connector = Connector::new(outbound_handshaker);
    let outbound_connection_maintainer = connection_maintainer::OutboundConnectionKeeper::new(
        config.clone(),
        new_connection_tx.clone(),
        make_connection_rx,
        address_book.clone(),
        outbound_connector,
    );

    let peer_set = PeerSet::new(new_connection_rx, syncer_wake);

    // Create semaphore for limiting inbound connections and monitoring
    let inbound_semaphore = Arc::new(tokio::sync::Semaphore::new(config.max_inbound_connections));

    let mut background_tasks = JoinSet::new();

    background_tasks.spawn(
        outbound_connection_maintainer
            .run()
            .instrument(Span::current()),
    );

    // Spawn inbound connection monitor task
    background_tasks.spawn(
        inbound_connection_monitor(
            Arc::clone(&inbound_semaphore),
            config.max_inbound_connections,
            config.p2p_port,
        )
        .instrument(tracing::info_span!("inbound_connection_monitor")),
    );

    background_tasks.spawn(
        inbound_server::inbound_server(
            new_connection_tx,
            inbound_handshaker,
            address_book.clone(),
            config,
            transport_config.server_config,
            inbound_semaphore,
        )
        .map(|res| {
            if let Err(e) = res {
                tracing::error!("Error in inbound connection listener: {e}");
            }

            tracing::info!("Inbound connection listener shutdown");
        })
        .instrument(Span::current()),
    );

    Ok(NetworkInterface {
        peer_set: Buffer::new(peer_set, 10).boxed_clone(),
        broadcast_svc,
        make_connection_tx,
        address_book: address_book.boxed_clone(),
        _background_tasks: Arc::new(background_tasks),
    })
}

/// The interface to Monero's P2P network on a certain [`NetworkZone`].
#[derive(Clone)]
pub struct NetworkInterface<N: NetworkZone> {
    /// A pool of free connected peers.
    peer_set: BoxCloneService<PeerSetRequest, PeerSetResponse<N>, tower::BoxError>,
    /// A [`Service`] that allows broadcasting to all connected peers.
    broadcast_svc: BroadcastSvc<N>,
    /// A channel to request extra connections.
    #[expect(dead_code, reason = "will be used eventually")]
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

    /// Starts the block downloader and returns a stream that will yield sequentially downloaded blocks.
    pub fn block_downloader<C>(
        &self,
        our_chain_service: C,
        config: BlockDownloaderConfig,
    ) -> BufferStream<BlockBatch>
    where
        C: Service<ChainSvcRequest<N>, Response = ChainSvcResponse<N>, Error = tower::BoxError>
            + Send
            + 'static,
        C::Future: Send + 'static,
    {
        block_downloader::download_blocks(self.peer_set.clone(), our_chain_service, config)
    }

    /// Returns the address book service.
    pub fn address_book(
        &self,
    ) -> BoxCloneService<AddressBookRequest<N>, AddressBookResponse<N>, tower::BoxError> {
        self.address_book.clone()
    }

    /// Borrows the `PeerSet`, for access to connected peers.
    pub fn peer_set(
        &mut self,
    ) -> &mut BoxCloneService<PeerSetRequest, PeerSetResponse<N>, tower::BoxError> {
        &mut self.peer_set
    }
}

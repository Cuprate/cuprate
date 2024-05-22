//! Cuprate's P2P Crate.
//!
//! This crate contains a [`ClientPool`](client_pool::ClientPool) which holds connected peers on a single [`NetworkZone`](monero_p2p::NetworkZone).
//!
//! This crate also contains the different routing methods that control how messages should be sent, i.e. broadcast to all,
//! or send to a single peer.
//!
#![allow(dead_code)]

use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use tower::buffer::Buffer;
use tracing::{instrument, Instrument, Span};

use monero_p2p::{CoreSyncSvc, NetworkZone, PeerRequestHandler};

mod broadcast;
mod client_pool;
pub mod config;
pub mod connection_maintainer;
mod constants;
mod inbound_server;
mod sync_states;

use crate::connection_maintainer::MakeConnectionRequest;
pub use config::P2PConfig;
use monero_p2p::client::Connector;

/// Initializes the P2P [`NetworkInterface`] for a specific [`NetworkZone`].
///
/// This function starts all the tasks to maintain connections/ accept connections/ make connections.
///
/// To use you must provide, a peer request handler, which is given to each connection  and a core sync service
/// which keeps track of the sync state of our node.
#[instrument(level="debug", name="net", skip_all, fields(zone=N::NAME))]
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

    // Use the default config. Changing the defaults affects tx fluff times, which could effect D++ so for now don't allow changing
    // this.
    let (broadcast_svc, outbound_mkr, inbound_mkr) =
        broadcast::init_broadcast_channels(broadcast::BroadcastConfig::default());

    let mut basic_node_data = config.basic_node_data();

    if !N::CHECK_NODE_ID {
        // TODO: make sure this is the value monerod sets for anon networks.
        basic_node_data.peer_id = 1;
    }

    let outbound_handshaker = monero_p2p::client::HandShaker::new(
        address_book.clone(),
        sync_states_svc.clone(),
        core_sync_svc.clone(),
        peer_req_handler.clone(),
        outbound_mkr,
        basic_node_data.clone(),
    );

    let inbound_handshaker = monero_p2p::client::HandShaker::new(
        address_book.clone(),
        sync_states_svc,
        core_sync_svc.clone(),
        peer_req_handler,
        inbound_mkr,
        basic_node_data,
    );

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

    tokio::spawn(
        outbound_connection_maintainer
            .run()
            .instrument(Span::current()),
    );
    tokio::spawn(
        inbound_server::inbound_server(client_pool.clone(), inbound_handshaker, config)
            .instrument(Span::current()),
    );

    Ok(NetworkInterface {
        pool: client_pool,
        broadcast_svc,
        top_block_watch,
        make_connection_tx,
    })
}

/// The interface to Monero's P2P network on a certain [`NetworkZone`].
pub struct NetworkInterface<N: NetworkZone> {
    /// A pool of free connected peers.
    pool: Arc<client_pool::ClientPool<N>>,
    /// A [`Service`](tower::Service) that allows broadcasting to all connected peers.
    broadcast_svc: broadcast::BroadcastSvc<N>,
    /// A [`watch`] channel that contains the highest seen cumulative difficulty and other info
    /// on that claimed chain.
    top_block_watch: watch::Receiver<sync_states::NewSyncInfo>,
    /// A channel to request extra connections.
    make_connection_tx: mpsc::Sender<MakeConnectionRequest>,
}

//! Cuprate's P2P Crate.
//!
//! This crate contains a `PeerSet` which holds connected peers on a single [`NetworkZone`](monero_p2p::NetworkZone).
//! The `PeerSet` has methods to get peers by direction (inbound/outbound) or by a custom method like a load balancing
//! algorithm.
//!
//! This crate also contains the different routing methods that control how messages should be sent, i.e. broadcast to all,
//! or send to a single peer.
//!

#![allow(dead_code)]

use rand::{random, Rng};
use tokio::sync::mpsc;
use tower::buffer::Buffer;

use monero_p2p::{AddressBook, CoreSyncSvc, NetworkZone, PeerRequestHandler};
use monero_wire::{common::PeerSupportFlags, BasicNodeData};

mod block_downloader;
pub mod broadcast;
pub mod config;
pub mod connection_maintainer;
mod constants;
mod network;
mod peer_set;
mod peer_sync_state;

use crate::network::P2PNetwork;
pub use config::P2PConfig;

pub async fn init_network<N: NetworkZone, CSync, ReqHdlr>(
    config: &P2PConfig,
    core_sync_svc: CSync,
    peer_request_hdlr: ReqHdlr,
) -> Result<P2PNetwork<N, impl AddressBook<N>>, tower::BoxError>
where
    CSync: CoreSyncSvc + Clone,
    ReqHdlr: PeerRequestHandler + Clone,
{
    let our_basic_node_data = make_basic_node_data::<N>(config);

    let addr_book =
        monero_address_book::init_address_book::<N>(config.address_book_config.clone()).await?;

    let (broadcast_svc, outbound_stream_mkr, _inbound_stream_mkr) =
        broadcast::init_broadcast_channels::<N>(&config.broadcast_config);

    let (peer_sync_svc, top_sync_data_watch) = peer_sync_state::PeerSyncSvc::<N>::new();

    let peer_sync_svc = Buffer::new(
        peer_sync_svc,
        config.max_outbound_connections + config.max_inbound_connections,
    );

    let outbound_handshaker = monero_p2p::client::HandShaker::new(
        addr_book.clone(),
        peer_sync_svc.clone(),
        core_sync_svc,
        peer_request_hdlr,
        outbound_stream_mkr,
        our_basic_node_data,
    );

    let outbound_connector = monero_p2p::client::Connector::new(outbound_handshaker);

    let (new_connection_tx, new_connection_rx) = mpsc::channel(config.outbound_connections);
    let (make_connection_tx, make_connection_rx) = mpsc::channel(5);

    let connection_maintainer = connection_maintainer::OutboundConnectionKeeper::new(
        &config,
        new_connection_tx,
        make_connection_rx,
        addr_book.clone(),
        outbound_connector,
    );

    tokio::spawn(connection_maintainer.run());

    let peer_set = peer_set::PeerSet::new(new_connection_rx, make_connection_tx);

    Ok(P2PNetwork::new(
        peer_set,
        broadcast_svc,
        addr_book,
        top_sync_data_watch,
    ))
}

fn make_basic_node_data<N: NetworkZone>(config: &P2PConfig) -> BasicNodeData {
    let peer_id = if N::CHECK_NODE_ID { random() } else { 1 };

    BasicNodeData {
        my_port: config.p2p_port as u32,
        network_id: config.network.network_id(),
        peer_id,
        support_flags: PeerSupportFlags::FLUFFY_BLOCKS,
        rpc_port: config.rpc_port,
        rpc_credits_per_hash: 0,
    }
}

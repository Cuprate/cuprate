//! Cuprate's P2P Crate.
//!
//! This crate contains a [`ClientPool`](client_pool::ClientPool) which holds connected peers on a single [`NetworkZone`](monero_p2p::NetworkZone).
//!
//! This crate also contains the different routing methods that control how messages should be sent, i.e. broadcast to all,
//! or send to a single peer.
//!
#![allow(dead_code)]

use std::sync::Arc;
use tokio::sync::watch;
use tower::buffer::Buffer;

use monero_p2p::NetworkZone;

mod broadcast;
mod client_pool;
pub mod config;
pub mod connection_maintainer;
mod constants;
mod sync_states;

pub use config::P2PConfig;

pub async fn initialize_network<N, R, CS>(
    peer_req_handler: R,
    core_sync_svc: CS,
    config: P2PConfig<N>,
) -> Result<NetworkInterface<N>, tower::BoxError>
where
    N: NetworkZone,
{
    let address_book = monero_address_book::init_address_book(config.address_book_config).await?;
    let address_book = Buffer::new(
        address_book,
        config.max_inbound_connections + config.outbound_connections,
    );

    let (sync_states_svc, top_block_watcher) = sync_states::PeerSyncSvc::new();

    // Use the default config. Changing the defaults effects tx fluff times, which could effect D++ so for now don't allow changing
    // this.
    let (broadcast_svc, outbound_mkr, inbound_mkr) =
        broadcast::init_broadcast_channels(&broadcast::BroadcastConfig::default());

    let mut basic_node_data = config.basic_node_data();

    if !N::CHECK_NODE_ID {
        // TODO: make sure this is the value monerod sets for anonn networks.
        basic_node_data.peer_id = 1;
    }

    let outbound_handshaker = monero_p2p::client::HandShaker::new(
        address_book.clone(),
        sync_states_svc,
        core_sync_svc,
        peer_req_handler,
        outbound_mkr,
        basic_node_data,
    );

    let inbound_handshaker = monero_p2p::client::HandShaker::new(
        address_book.clone(),
        sync_states_svc,
        core_sync_svc,
        peer_req_handler,
        inbound_mkr,
        basic_node_data,
    );
    
    let outb
    
}

pub struct NetworkInterface<N: NetworkZone> {
    pool: Arc<client_pool::ClientPool<N>>,
    broadcast_svc: broadcast::BroadcastSvc<N>,
    top_block_watch: watch::Receiver<sync_states::NewSyncInfo>,
}

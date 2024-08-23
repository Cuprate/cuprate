//! P2P
//!
//! Will handle initiating the P2P and contains a protocol request handler.

use cuprate_p2p::AddressBookConfig;
use cuprate_p2p_core::Network;
use std::time::Duration;

pub mod core_sync_svc;
pub mod request_handler;

pub fn dummy_config<N: cuprate_p2p_core::NetworkZone>() -> cuprate_p2p::P2PConfig<N> {
    cuprate_p2p::P2PConfig {
        network: Network::Mainnet,
        outbound_connections: 64,
        extra_outbound_connections: 0,
        max_inbound_connections: 0,
        gray_peers_percent: 0.7,
        server_config: None,
        p2p_port: 0,
        rpc_port: 0,
        address_book_config: AddressBookConfig {
            max_white_list_length: 1000,
            max_gray_list_length: 5000,
            peer_store_file: "p2p_state.bin".into(),
            peer_save_period: Duration::from_secs(60),
        },
    }
}

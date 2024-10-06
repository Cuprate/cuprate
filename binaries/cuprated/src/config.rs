//! cuprated config
use serde::{Deserialize, Serialize};
use cuprate_helper::network::Network;
use cuprate_p2p_core::ClearNet;

mod sections;

use sections::P2PConfig;

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    network: Network,
    
    p2p: P2PConfig,
}

impl Config {
    fn clearnet_p2p_config(&self) -> cuprate_p2p::P2PConfig<ClearNet> {
        cuprate_p2p::P2PConfig {
            network: self.network,
            outbound_connections: self.p2p.clear_net.general.outbound_connections,
            extra_outbound_connections: self.p2p.clear_net.general.extra_outbound_connections,
            max_inbound_connections:self.p2p.clear_net.general.max_inbound_connections,
            gray_peers_percent: self.p2p.clear_net.general.gray_peers_percent,
            server_config: Some(self.p2p.clear_net.server.clone()),
            p2p_port: self.p2p.clear_net.general.p2p_port,
            rpc_port: 0,
            address_book_config: self.p2p.clear_net.general.address_book_config.clone(),
        }
    }
}

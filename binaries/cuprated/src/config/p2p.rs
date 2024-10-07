use cuprate_address_book::AddressBookConfig;
use cuprate_helper::network::Network;
use cuprate_p2p_core::ClearNetServerCfg;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

pub fn clear_net_seed_nodes(network: Network) -> Vec<SocketAddr> {
    let seeds = match network {
        Network::Mainnet => [
            "176.9.0.187:18080",
            "88.198.163.90:18080",
            "66.85.74.134:18080",
            "51.79.173.165:18080",
            "192.99.8.110:18080",
            "37.187.74.171:18080",
            "77.172.183.193:18080",
        ]
        .as_slice(),
        Network::Stagenet => [
            "176.9.0.187:38080",
            "51.79.173.165:38080",
            "192.99.8.110:38080",
            "37.187.74.171:38080",
            "77.172.183.193:38080",
        ]
        .as_slice(),
        Network::Testnet => [
            "176.9.0.187:28080",
            "51.79.173.165:28080",
            "192.99.8.110:28080",
            "37.187.74.171:28080",
            "77.172.183.193:28080",
        ]
        .as_slice(),
    };

    seeds
        .into_iter()
        .map(|&s| str::parse(s))
        .collect::<Result<_, _>>()
        .unwrap()
}

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct P2PConfig {
    pub clear_net: ClearNetConfig,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct ClearNetConfig {
    pub server: ClearNetServerCfg,
    #[serde(flatten)]
    pub general: SharedNetConfig,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct SharedNetConfig {
    /// The number of outbound connections to make and try keep.
    pub outbound_connections: usize,
    /// The amount of extra connections we can make if we are under load from the rest of Cuprate.
    pub extra_outbound_connections: usize,
    /// The maximum amount of inbound connections
    pub max_inbound_connections: usize,
    pub gray_peers_percent: f64,
    /// port to use to accept p2p connections.
    pub p2p_port: u16,
    /// The address book config.
    pub address_book_config: AddressBookConfig,
}

impl Default for SharedNetConfig {
    fn default() -> Self {
        Self {
            outbound_connections: 64,
            extra_outbound_connections: 8,
            max_inbound_connections: 128,
            gray_peers_percent: 0.7,
            p2p_port: 18080,
            address_book_config: AddressBookConfig::default(),
        }
    }
}

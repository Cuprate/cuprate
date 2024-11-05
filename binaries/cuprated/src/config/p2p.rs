use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use cuprate_address_book::AddressBookConfig;
use cuprate_helper::network::Network;
use cuprate_p2p::block_downloader::BlockDownloaderConfig;
use cuprate_p2p_core::ClearNetServerCfg;

/// P2P config.
#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct P2PConfig {
    /// Clear-net config.
    pub clear_net: ClearNetConfig,
    /// Block downloader config.
    pub block_downloader: BlockDownloaderConfig,
}

/// The config values for P2P clear-net.
#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct ClearNetConfig {
    /// The server config.
    pub server: ClearNetServerCfg,
    #[serde(flatten)]
    pub general: SharedNetConfig,
}

/// Network config values shared between all network zones.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct SharedNetConfig {
    /// The number of outbound connections to make and try keep.
    pub outbound_connections: usize,
    /// The amount of extra connections we can make if we are under load from the rest of Cuprate.
    pub extra_outbound_connections: usize,
    /// The maximum amount of inbound connections
    pub max_inbound_connections: usize,
    /// The percent of connections that should be to peers we haven't connected to before.
    pub gray_peers_percent: f64,
    /// port to use to accept p2p connections.
    pub p2p_port: u16,
    /// The address book config.
    address_book_config: AddressBookConfig,
}

impl SharedNetConfig {
    /// Returns the [`AddressBookConfig`].
    pub fn address_book_config(&self, network: Network) -> AddressBookConfig {
        // HACK: we add the network here so we don't need to define another address book config.
        let mut address_book_config = self.address_book_config.clone();
        address_book_config
            .peer_store_directory
            .push(network.to_string());

        address_book_config
    }
}

impl Default for SharedNetConfig {
    fn default() -> Self {
        Self {
            outbound_connections: 64,
            extra_outbound_connections: 8,
            max_inbound_connections: 128,
            gray_peers_percent: 0.7,
            p2p_port: 0,
            address_book_config: AddressBookConfig::default(),
        }
    }
}

/// Seed nodes for [`ClearNet`](cuprate_p2p_core::ClearNet).
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
        .iter()
        .map(|s| s.parse())
        .collect::<Result<_, _>>()
        .unwrap()
}

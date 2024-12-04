use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    path::Path,
    time::Duration,
};

use serde::{Deserialize, Serialize};

use cuprate_helper::{fs::address_book_path, network::Network};

/// P2P config.
#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct P2PConfig {
    /// Clear-net config.
    pub clear_net: ClearNetConfig,
    /// Block downloader config.
    pub block_downloader: BlockDownloaderConfig,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct BlockDownloaderConfig {
    /// The size in bytes of the buffer between the block downloader and the place which
    /// is consuming the downloaded blocks.
    pub buffer_bytes: usize,
    /// The size of the in progress queue (in bytes) at which we stop requesting more blocks.
    pub in_progress_queue_bytes: usize,
    /// The [`Duration`] between checking the client pool for free peers.
    pub check_client_pool_interval: Duration,
    /// The target size of a single batch of blocks (in bytes).
    pub target_batch_bytes: usize,
}

impl From<BlockDownloaderConfig> for cuprate_p2p::block_downloader::BlockDownloaderConfig {
    fn from(value: BlockDownloaderConfig) -> Self {
        Self {
            buffer_bytes: value.buffer_bytes,
            in_progress_queue_bytes: value.in_progress_queue_bytes,
            check_client_pool_interval: value.check_client_pool_interval,
            target_batch_bytes: value.target_batch_bytes,
            initial_batch_len: 1,
        }
    }
}

impl Default for BlockDownloaderConfig {
    fn default() -> Self {
        Self {
            buffer_bytes: 50_000_000,
            in_progress_queue_bytes: 50_000_000,
            check_client_pool_interval: Duration::from_secs(30),
            target_batch_bytes: 5_000_000,
        }
    }
}

/// The config values for P2P clear-net.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct ClearNetConfig {
    /// The server config.
    pub listen_on: IpAddr,
    #[serde(flatten)]
    pub general: SharedNetConfig,
}

impl Default for ClearNetConfig {
    fn default() -> Self {
        Self {
            listen_on: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            general: Default::default(),
        }
    }
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
    pub fn address_book_config(
        &self,
        cache_dir: &Path,
        network: Network,
    ) -> cuprate_address_book::AddressBookConfig {
        cuprate_address_book::AddressBookConfig {
            max_white_list_length: self.address_book_config.max_white_list_length,
            max_gray_list_length: self.address_book_config.max_gray_list_length,
            peer_store_directory: address_book_path(cache_dir, network),
            peer_save_period: self.address_book_config.peer_save_period,
        }
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

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct AddressBookConfig {
    max_white_list_length: usize,
    max_gray_list_length: usize,
    peer_save_period: Duration,
}

impl Default for AddressBookConfig {
    fn default() -> Self {
        Self {
            max_white_list_length: 1_000,
            max_gray_list_length: 5_000,
            peer_save_period: Duration::from_secs(30),
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

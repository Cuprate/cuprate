use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    path::Path,
    time::Duration,
};

use serde::{Deserialize, Serialize};

use cuprate_helper::{fs::address_book_path, network::Network};

use super::macros::config_struct;

config_struct! {
    /// P2P config.
    #[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
    #[serde(deny_unknown_fields, default)]
    pub struct P2PConfig {
        #[child = true]
        /// The clear-net P2P config.
        pub clear_net: ClearNetConfig,

        #[child = true]
        /// Block downloader config.
        ///
        /// The block downloader handles downloading old blocks from peers when we are behind.
        pub block_downloader: BlockDownloaderConfig,
    }
}

config_struct! {
    #[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
    #[serde(deny_unknown_fields, default)]
    pub struct BlockDownloaderConfig {
        #[comment_out = true]
        /// The size in bytes of the buffer between the block downloader
        /// and the place which is consuming the downloaded blocks (`cuprated`).
        ///
        /// This value is an absolute maximum,
        /// once this is reached the block downloader will pause.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 1_000_000_000, 5_500_000_000, 500_000_000
        pub buffer_bytes: usize,

        #[comment_out = true]
        /// The size of the in progress queue (in bytes)
        /// at which cuprated stops requesting more blocks.
        ///
        /// The value is _NOT_ an absolute maximum,
        /// the in-progress queue could get much larger.
        /// This value is only the value cuprated stops requesting more blocks,
        /// if cuprated still has requests in progress,
        /// it will still accept the response and add the blocks to the queue.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 500_000_000, 1_000_000_000,
        pub in_progress_queue_bytes: usize,

        #[inline = true]
        /// The duration between checking the client pool for free peers.
        ///
        /// Type     | Duration
        /// Examples | { secs = 30, nanos = 0 }, { secs = 35, nano = 123 }
        pub check_client_pool_interval: Duration,

        #[comment_out = true]
        /// The target size of a single batch of blocks (in bytes).
        ///
        /// This value must be below 100_000,000,
        /// it is not recommended to set it above 30_000_000.
        ///
        /// Type         | Number
        /// Valid values | 0..100_000,000
        pub target_batch_bytes: usize,
    }
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
            buffer_bytes: 1_000_000_000,
            in_progress_queue_bytes: 500_000_000,
            check_client_pool_interval: Duration::from_secs(30),
            target_batch_bytes: 15_000_000,
        }
    }
}

config_struct! {
    /// The config values for P2P clear-net.
    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    #[serde(deny_unknown_fields, default)]
    pub struct ClearNetConfig {
        /// The IP address to bind and listen for connections on.
        ///
        /// Type     | IPv4/IPv6 address
        /// Examples | "0.0.0.0", "192.168.1.50", "::"
        pub listen_on: IpAddr,

        #[flatten = true]
        /// Shared config values.
        ##[serde(flatten)]
        pub general: SharedNetConfig,
    }
}

impl Default for ClearNetConfig {
    fn default() -> Self {
        Self {
            listen_on: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            general: Default::default(),
        }
    }
}

config_struct! {
    /// Network config values shared between all network zones.
    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    #[serde(deny_unknown_fields, default)]
    pub struct SharedNetConfig {
        #[comment_out = true]
        /// The number of outbound connections to make and try keep.
        ///
        /// It's recommended to keep this value above 12.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 12, 32, 64, 100, 500
        pub outbound_connections: usize,

        #[comment_out = true]
        /// The amount of extra connections to make if cuprated is under load.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 0, 12, 32, 64, 100, 500
        pub extra_outbound_connections: usize,

        #[comment_out = true]
        /// The maximum amount of inbound connections to allow.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 0, 12, 32, 64, 100, 500
        pub max_inbound_connections: usize,

        #[comment_out = true]
        /// The percent of connections that should be
        /// to peers that haven't connected to before.
        ///
        /// 0.0 is 0%.
        /// 1.0 is 100%.
        ///
        /// Type         | Floating point number
        /// Valid values | 0.0..1.0
        /// Examples     | 0.0, 0.5, 0.123, 0.999, 1.0
        pub gray_peers_percent: f64,

        /// The port to use to accept incoming P2P connections.
        ///
        /// Setting this to 0 will disable incoming P2P connections.
        ///
        /// Type         | Number
        /// Valid values | 0..65534
        /// Examples     | 18080, 9999, 5432
        pub p2p_port: u16,

        #[child = true]
        /// The address book config.
        pub address_book_config: AddressBookConfig,
    }
}

impl SharedNetConfig {
    /// Returns the [`cuprate_address_book::AddressBookConfig`].
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
            outbound_connections: 32,
            extra_outbound_connections: 8,
            max_inbound_connections: 128,
            gray_peers_percent: 0.7,
            p2p_port: 18080,
            address_book_config: AddressBookConfig::default(),
        }
    }
}

config_struct! {
    /// The addressbook config exposed to users.
    #[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
    #[serde(deny_unknown_fields, default)]
    pub struct AddressBookConfig {
        /// The size of the white peer list.
        ///
        /// The white list holds peers that have been connected to before.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 1000, 500, 241
        pub max_white_list_length: usize,

        /// The size of the gray peer list.
        ///
        /// The gray peer list holds peers that have been
        /// told about but not connected to cuprated.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 1000, 500, 241
        pub max_gray_list_length: usize,

        #[inline = true]
        /// The time period between address book saves.
        ///
        /// Type     | Duration
        /// Examples | { secs = 90, nanos = 0 }, { secs = 100, nano = 123 }
        pub peer_save_period: Duration,
    }
}

impl Default for AddressBookConfig {
    fn default() -> Self {
        Self {
            max_white_list_length: 1_000,
            max_gray_list_length: 5_000,
            peer_save_period: Duration::from_secs(90),
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

use serde::{Deserialize, Serialize};
use std::cmp::{max, min};
use std::{
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    path::Path,
    time::Duration,
};

use cuprate_helper::{fs::address_book_path, network::Network};
use cuprate_p2p::config::TransportConfig;
use cuprate_p2p_core::{
    transports::{Tcp, TcpServerConfig},
    ClearNet, NetworkZone, Tor, Transport,
};
use cuprate_wire::OnionAddr;

use crate::{p2p::ProxySettings, tor::TorMode};

use super::{default::DefaultOrCustom, macros::config_struct};

use cuprate_helper::cast::u64_to_usize;
#[cfg(feature = "arti")]
use {
    arti_client::{
        config::onion_service::{OnionServiceConfig, OnionServiceConfigBuilder},
        TorClient, TorClientBuilder, TorClientConfig,
    },
    cuprate_p2p_transport::{Arti, ArtiClientConfig, ArtiServerConfig, Socks, SocksClientConfig},
    tor_rtcompat::PreferredRuntime,
};

config_struct! {
    /// P2P config.
    #[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
    #[serde(deny_unknown_fields, default)]
    pub struct P2PConfig {
        #[child = true]
        /// The clear-net P2P config.
        pub clear_net: ClearNetConfig,

        #[child = true]
        /// The tor-net P2P config.
        pub tor_net: TorNetConfig,

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
        pub buffer_bytes: DefaultOrCustom<usize>,

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
        pub in_progress_queue_bytes: DefaultOrCustom<usize>,

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
        let mut info = sysinfo::System::new();
        info.refresh_memory();

        let buffer_mem = u64_to_usize(min(info.total_memory() / 5, 1024 * 1024 * 1024));

        Self {
            buffer_bytes: *value.buffer_bytes.value(&buffer_mem),
            in_progress_queue_bytes: *value.in_progress_queue_bytes.value(&(buffer_mem / 2)),
            check_client_pool_interval: value.check_client_pool_interval,
            target_batch_bytes: value.target_batch_bytes,
            initial_batch_len: 1,
        }
    }
}

impl Default for BlockDownloaderConfig {
    fn default() -> Self {
        Self {
            buffer_bytes: DefaultOrCustom::Default,
            in_progress_queue_bytes: DefaultOrCustom::Default,
            check_client_pool_interval: Duration::from_secs(30),
            target_batch_bytes: 15_000_000,
        }
    }
}

config_struct! {
    Shared {
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

        /// The port bind to this network zone.
        ///
        /// This port will be bind to if the incoming P2P
        /// server for this zone has been enabled.
        ///
        /// Type         | Number or "Default"
        /// Valid values | 0..65534, "Default"
        /// Examples     | 18080, 9999, 5432
        pub p2p_port: DefaultOrCustom<u16>,

        #[child = true]
        /// The address book config.
        pub address_book_config: AddressBookConfig,
    }

    /// The config values for P2P clear-net.
    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    #[serde(deny_unknown_fields, default)]
    pub struct ClearNetConfig {

        /// Enable IPv4 inbound server.
        ///
        /// The inbound server will listen on port `p2p.clear_net.p2p_port`.
        /// Setting this to `false` will disable incoming IPv4 P2P connections.
        ///
        /// Type         | boolean
        /// Valid values | false, true
        /// Examples     | false
        pub enable_inbound: bool,

        /// The IPv4 address to bind and listen for connections on.
        ///
        /// Type     | IPv4 address
        /// Examples | "0.0.0.0", "192.168.1.50"
        pub listen_on: Ipv4Addr,

        /// Enable IPv6 inbound server.
        ///
        /// The inbound server will listen on port `p2p.clear_net.p2p_port`.
        /// Setting this to `false` will disable incoming IPv6 P2P connections.
        ///
        /// Type         | boolean
        /// Valid values | false, true
        /// Examples     | false
        pub enable_inbound_v6: bool,

        /// The IPv6 address to bind and listen for connections on.
        ///
        /// Type     | IPv6 address
        /// Examples | "::", "2001:0db8:85a3:0000:0000:8a2e:0370:7334"
        pub listen_on_v6: Ipv6Addr,

        #[comment_out = true]
        /// The proxy to use for outgoing P2P connections
        ///
        /// This setting can only take "Tor" at the moment.
        /// This will anonymise clearnet connections through Tor.
        ///
        /// Setting this to "" (an empty string) will disable the proxy.
        ///
        /// Enabling this setting will disable inbound connections.
        ///
        /// Type         | String
        /// Valid values | "Tor"
        /// Examples     | "Tor"
        pub proxy: ProxySettings,
    }

    /// The config values for P2P tor.
    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    #[serde(deny_unknown_fields, default)]
    pub struct TorNetConfig {

        #[comment_out = true]
        /// Enable the Tor P2P network.
        ///
        /// Type         | boolean
        /// Valid values | false, true
        /// Examples     | false
        pub enabled: bool,

        #[comment_out = true]
        /// Enable Tor inbound onion server.
        ///
        /// In Arti mode, setting this to `true` will enable Arti's onion service for accepting inbound
        /// Tor P2P connections. The keypair and therefore onion address is generated randomly on first run.
        ///
        /// In Daemon mode, setting this to `true` will enable a TCP server listening for inbound connections
        /// from your Tor daemon. Refer to the `tor.anonymous_inbound` and `tor.listening_addr` field for onion address
        /// and listening configuration.
        ///
        /// The server will listen on port `p2p.tor_net.p2p_port`
        ///
        /// Type         | boolean
        /// Valid values | false, true
        /// Examples     | false
        pub inbound_onion: bool,
    }
}

/// Gets the port to listen on for p2p connections.
pub const fn p2p_port(setting: DefaultOrCustom<u16>, network: Network) -> u16 {
    match setting {
        DefaultOrCustom::Default => match network {
            Network::Mainnet => 18080,
            Network::Stagenet => 38080,
            Network::Testnet => 28080,
        },
        DefaultOrCustom::Custom(port) => port,
    }
}

impl ClearNetConfig {
    /// Gets the transport config for [`ClearNet`] over [`Tcp`].
    pub fn tcp_transport_config(&self, network: Network) -> TransportConfig<ClearNet, Tcp> {
        let server_config = if self.enable_inbound {
            let mut sc = TcpServerConfig::default();
            sc.ipv4 = Some(self.listen_on);
            sc.ipv6 = self.enable_inbound_v6.then_some(self.listen_on_v6);
            sc.port = p2p_port(self.p2p_port, network);
            Some(sc)
        } else {
            None
        };

        TransportConfig {
            client_config: (),
            server_config,
        }
    }
}

impl Default for ClearNetConfig {
    fn default() -> Self {
        Self {
            p2p_port: DefaultOrCustom::Default,
            enable_inbound: true,
            listen_on: Ipv4Addr::UNSPECIFIED,
            enable_inbound_v6: false,
            listen_on_v6: Ipv6Addr::UNSPECIFIED,
            proxy: ProxySettings::Socks(String::new()),
            outbound_connections: 32,
            extra_outbound_connections: 8,
            max_inbound_connections: 128,
            gray_peers_percent: 0.7,
            address_book_config: AddressBookConfig::default(),
        }
    }
}

impl Default for TorNetConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            inbound_onion: false,
            p2p_port: DefaultOrCustom::Default,
            outbound_connections: 12,
            extra_outbound_connections: 2,
            max_inbound_connections: 128,
            gray_peers_percent: 0.7,
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

impl AddressBookConfig {
    /// Returns the [`cuprate_address_book::AddressBookConfig`].
    pub fn address_book_config<Z: NetworkZone>(
        &self,
        cache_dir: &Path,
        network: Network,
        our_own_address: Option<Z::Addr>,
    ) -> cuprate_address_book::AddressBookConfig<Z> {
        cuprate_address_book::AddressBookConfig {
            max_white_list_length: self.max_white_list_length,
            max_gray_list_length: self.max_gray_list_length,
            peer_store_directory: address_book_path(cache_dir, network),
            peer_save_period: self.peer_save_period,
            our_own_address,
        }
    }
}

/// Seed nodes for [`ClearNet`].
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

/// Seed nodes for `Tor`.
pub fn tor_net_seed_nodes(network: Network) -> Vec<OnionAddr> {
    let seeds = match network {
        Network::Mainnet => [
            "zbjkbsxc5munw3qusl7j2hpcmikhqocdf4pqhnhtpzw5nt5jrmofptid.onion:18083",
            "qz43zul2x56jexzoqgkx2trzwcfnr6l3hbtfcfx54g4r3eahy3bssjyd.onion:18083",
            "plowsof3t5hogddwabaeiyrno25efmzfxyro2vligremt7sxpsclfaid.onion:18083",
            "plowsoffjexmxalw73tkjmf422gq6575fc7vicuu4javzn2ynnte6tyd.onion:18083",
            "plowsofe6cleftfmk2raiw5h2x66atrik3nja4bfd3zrfa2hdlgworad.onion:18083",
            "aclc4e2jhhtr44guufbnwk5bzwhaecinax4yip4wr4tjn27sjsfg6zqd.onion:18083",
        ]
        .as_slice(),
        Network::Stagenet | Network::Testnet => [].as_slice(),
    };

    seeds
        .iter()
        .map(|s| s.parse())
        .collect::<Result<_, _>>()
        .unwrap()
}

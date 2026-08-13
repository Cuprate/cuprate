use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

use serde::{Deserialize, Serialize};

use cuprate_helper::network::Network;

use super::{default::DefaultOrCustom, macros::config_struct};

config_struct! {
    /// RPC config.
    #[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields, default)]
    pub struct RpcConfig {
        #[child = true]
        /// Configuration for the unrestricted RPC server.
        pub unrestricted: UnrestrictedRpcConfig,

        #[child = true]
        /// Configuration for the restricted RPC server.
        pub restricted: RestrictedRpcConfig,
    }
}

config_struct! {
    Shared {
        /// The address the RPC server will listen on.
        ///
        /// Type     | IPv4/IPv6 address
        /// Examples | "", "127.0.0.1", "192.168.1.50"
        pub address: IpAddr,

        /// The port the RPC server will listen on.
        ///
        /// Type         | Number or "Default"
        /// Valid values | 0..65534, "Default"
        /// Examples     | 18081, 18089, 5432
        pub port: DefaultOrCustom<u16>,

        /// Toggle the RPC server.
        ///
        /// If `true` the RPC server will be enabled.
        /// If `false` the RPC server will be disabled.
        ///
        /// Type     | boolean
        /// Examples | true, false
        pub enable: bool,

        #[comment_out = true]
        /// If a request is above this byte limit, it will be rejected.
        ///
        /// Setting this to `0` will disable the limit.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 0 (no limit), 5242880 (5MB), 10485760 (10MB)
        pub request_byte_limit: usize,

        #[comment_out = true]
        /// Maximum amount of RPC connections allowed by a single public IP address.
        ///
        /// Setting this to `0` will disable the limit.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 0 (no limit), 2, 4
        pub public_ip_connection_limit: usize,

        #[comment_out = true]
        /// Maximum amount of RPC connections allowed by a single private IP address.
        ///
        /// Setting this to `0` will disable the limit.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 0 (no limit), 16, 100
        pub private_ip_connection_limit: usize,

        #[comment_out = true]
        /// Maximum amount of RPC connections allowed by a loopback address.
        ///
        /// Setting this to `0` will disable the limit.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 0 (no limit), 67
        pub loopback_connection_limit: usize,

        #[comment_out = true]
        /// Maximum amount of RPC connections allowed globally.
        ///
        /// Setting this to `0` will disable the limit.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 0 (no limit), 16, 300
        pub total_connection_limit: usize,

        #[comment_out = true]
        /// The list of IP addresses that are excluded from
        /// all RPC connection limits.
        ///
        /// This can be useful if you are using a reverse proxy
        /// in front of this node.
        ///
        /// Type     | IPv4/IPv6 address
        /// Examples | "", "127.0.0.1", "192.168.1.50"
        pub excluded_ips_connection_limit: Vec<IpAddr>,

        #[comment_out = true]
        /// The maximum size budget of a single IP address, in bytes.
        ///
        /// Every IP address has a size budget, the size of each
        /// response it receives is subtracted from it, and it is
        /// refilled by `income_size` every second. Requests received
        /// while this budget is exhausted are rejected with
        /// `429 Too Many Requests`.
        ///
        /// Type         | Number (bytes)
        /// Valid values | >= 0 (0 rejects all requests)
        /// Examples     | 536870912 (512MiB), 1073741824 (1GiB)
        pub max_budget_size: u64,

        #[comment_out = true]
        /// The amount of bytes per second refilled to the size
        /// budget of every IP address, capped at `max_budget_size`.
        ///
        /// Type         | Number (bytes per second)
        /// Valid values | >= 0 (0 means no restriction)
        /// Examples     | 33554432 (32MiB/s), 67108864 (64MiB/s)
        pub income_size: u64,

        #[comment_out = true]
        /// The maximum processing time budget of a single IP address,
        /// in milliseconds.
        ///
        /// Every IP address has a processing time budget: the processing
        /// time of each of its requests is subtracted from it, and it is
        /// refilled by `income_time` every second. Requests received
        /// while this budget is exhausted are rejected with
        /// `429 Too Many Requests`.
        ///
        /// Type         | Number (milliseconds)
        /// Valid values | >= 0 (0 means no restrictions)
        /// Examples     | 30000 (30s), 60000 (1m)
        pub max_budget_time: u64,

        #[comment_out = true]
        /// The amount of milliseconds of processing time refilled per sec
        /// to the processing time budget of every IP address, capped
        /// at `max_budget_time`.
        ///
        /// Type         | Number (milliseconds per second)
        /// Valid values | >= 0
        /// Examples     | 250, 500
        pub income_time: u64,

        #[comment_out = true]
        /// Maximum amount of blocks endpoint requests handled simultaneously.
        ///
        /// The blocks endpoints are `/get_blocks.bin`, `/get_blocks_by_height.bin`
        /// (and their aliases), and `/json_rpc`'s `get_block` method.
        ///
        /// Additional requests are put on hold for up to
        /// `blocks_semaphore_queue_wait` milliseconds before being rejected
        /// with `503 Service Unavailable` and a `Retry-After` header.
        ///
        /// Setting this to `0` will disable the limit.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 0 (no limit), 4, 16
        pub blocks_semaphore_limit: u64,

        #[comment_out = true]
        /// The maximum amount of time, in milliseconds, a blocks endpoint
        /// request is put on hold waiting for a concurrency slot before
        /// being rejected with `503 Service Unavailable`.
        ///
        /// Type     | Duration
        /// Examples | { secs = 10, nanos = 0 }, { secs = 29, nano = 123 }
        pub blocks_semaphore_queue_wait: Duration,

        /// The time period during which the node can try sending data.
        /// If a send operation do not complete within this Duration,
        /// the connection is dropped.
        ///
        /// Type     | Duration
        /// Examples | { secs = 10, nanos = 0 }, { secs = 29, nano = 123 }
        pub send_timeout: Duration,

        /// The time period during which the node wait receiving data.
        /// If a receive operation do not complete within this Duration,
        /// the connection is dropped.
        ///
        /// Type     | Duration
        /// Examples | { secs = 10, nanos = 0 }, { secs = 29, nano = 123 }
        pub read_timeout: Duration,

        // TODO: <https://github.com/Cuprate/cuprate/issues/445>
    }

    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields, default)]
    pub struct UnrestrictedRpcConfig {
        /// Allow the unrestricted RPC server to be public.
        ///
        /// ⚠️ WARNING ⚠️
        /// -------------
        /// Unrestricted RPC should almost never be made available
        /// to the wider internet. If the unrestricted address
        /// is a non-local address, `cuprated` will crash,
        /// unless this setting is set to `true`.
        ///
        /// Type         | boolean
        /// Valid values | true, false
        pub i_know_what_im_doing_allow_public_unrestricted_rpc: bool,
    }

    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields, default)]
    pub struct RestrictedRpcConfig {
        /// Advertise the restricted RPC port.
        ///
        /// Setting this to `true` will make `cuprated`
        /// share the restricted RPC server's port
        /// publicly to the P2P network.
        ///
        /// Type         | boolean
        /// Valid values | true, false
        pub advertise: bool,
    }
}

impl Default for UnrestrictedRpcConfig {
    fn default() -> Self {
        Self {
            i_know_what_im_doing_allow_public_unrestricted_rpc: false,
            address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: DefaultOrCustom::Default,
            enable: true,
            request_byte_limit: 0,
            public_ip_connection_limit: 0,
            private_ip_connection_limit: 0,
            loopback_connection_limit: 0,
            total_connection_limit: 0,
            excluded_ips_connection_limit: Vec::new(),
            max_budget_size: i64::MAX as u64,
            income_size: i64::MAX as u64,
            max_budget_time: i64::MAX as u64,
            income_time: i64::MAX as u64,
            blocks_semaphore_limit: 0,
            blocks_semaphore_queue_wait: Duration::from_secs(0),
            send_timeout: Duration::from_secs(5),
            read_timeout: Duration::from_secs(30),
        }
    }
}

impl Default for RestrictedRpcConfig {
    fn default() -> Self {
        Self {
            advertise: false,
            address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            port: DefaultOrCustom::Default,
            enable: false,
            // 1 megabyte.
            // <https://github.com/monero-project/monero/blob/3b01c490953fe92f3c6628fa31d280a4f0490d28/src/cryptonote_config.h#L134>
            request_byte_limit: 1024 * 1024,
            public_ip_connection_limit: 3,
            private_ip_connection_limit: 25,
            loopback_connection_limit: 50,
            total_connection_limit: 100,
            excluded_ips_connection_limit: Vec::new(),
            max_budget_size: 512 * 1024 * 1024,
            income_size: 32 * 1024 * 1024,
            max_budget_time: 30_000,
            income_time: 250,
            blocks_semaphore_limit: 30, // 6GB
            blocks_semaphore_queue_wait: Duration::from_secs(1),
            send_timeout: Duration::from_secs(5),
            read_timeout: Duration::from_secs(30),
        }
    }
}

/// Gets the port to listen on for restricted RPC connections.
pub const fn restricted_rpc_port(config: DefaultOrCustom<u16>, network: Network) -> u16 {
    match config {
        DefaultOrCustom::Default => match network {
            Network::Mainnet | Network::FakeChain => 18089,
            Network::Stagenet => 38089,
            Network::Testnet => 28089,
        },
        DefaultOrCustom::Custom(port) => port,
    }
}

/// Gets the port to listen on for unrestricted RPC connections.
pub const fn unrestricted_rpc_port(config: DefaultOrCustom<u16>, network: Network) -> u16 {
    match config {
        DefaultOrCustom::Default => match network {
            Network::Mainnet | Network::FakeChain => 18081,
            Network::Stagenet => 38081,
            Network::Testnet => 28081,
        },
        DefaultOrCustom::Custom(port) => port,
    }
}

impl RestrictedRpcConfig {
    /// Return the restricted RPC port for P2P if available and public.
    pub const fn port_for_p2p(&self, network: Network) -> u16 {
        if self.advertise && self.enable {
            restricted_rpc_port(self.port, network)
        } else {
            0
        }
    }
}

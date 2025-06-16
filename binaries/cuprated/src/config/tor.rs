use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use cuprate_helper::fs::CUPRATE_DATA_DIR;

use crate::{config::macros::config_struct, tor::TorMode};

config_struct! {
    /// Arti config
    #[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields, default)]
    #[allow(rustdoc::broken_intra_doc_links)]
    pub struct ArtiConfig {
        /// Path to the arti state directory.
        ///
        /// Type         | String
        /// Valid values | false, true
        /// Examples     | false
        pub directory_path: PathBuf,

        /// Enable isolated circuits for Arti.
        ///
        /// If set, Arti will use different tor circuits for each connections. This can
        /// cause stability issues if the connection count is important.
        ///
        /// Type         | boolean
        /// Valid values | false, true
        /// Examples     | false
        pub isolated_circuit: bool,

        /// Enable PoW security for Arti.
        ///
        /// If set, Arti will enforce an EquiX PoW to be resolved for
        /// other nodes to complete a rendez-vous request. This is a
        /// DDoS mitigation and is only enabled during heavy load.
        ///
        /// Type         | boolean
        /// Valid values | false, true
        /// Examples     | false
        pub onion_service_pow: bool,
    }

    /// Tor config
    #[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields, default)]
    #[allow(rustdoc::broken_intra_doc_links)]
    pub struct TorDaemonConfig {
        /// The IP address and port of the external Tor daemon to use for outgoing connections.
        ///
        /// Type     | Socket address
        /// Examples | "[::1]:9050", "127.0.0.1:9050"
        pub address: SocketAddr,

        #[comment_out = true]
        /// Enable inbound connections for Daemon mode
        ///
        /// This string specify the onion address that should be advertized to the Tor network
        /// and that your daemon should be expecting connections from.
        ///
        /// When this is set, `p2p.tor_net.p2p_port` is not used for listening but as the source
        /// port of your hidden service in your torrc configuration file. For setting Cuprate's
        /// listening port see `tor.daemon_listening_port` field
        ///
        /// This setting is ignored in `Arti` mode.
        ///
        /// Type         | String
        /// Valid values | "<56 character domain>.onion"
        /// Examples     | "monerotoruzizulg5ttgat2emf4d6fbmiea25detrmmy7erypseyteyd.onion"
        pub anonymous_inbound: String,

        /// The IP address to bind and listen on for anonymous inbound connections from Tor Daemon.
        ///
        /// This setting is only took into account if `p2p.tor_net.enabled` is set to "Daemon".
        ///
        /// Type     | IP address
        /// Examples | "0.0.0.0", "192.168.1.50", "::", "2001:0db8:85a3:0000:0000:8a2e:0370:7334"
        pub listening_ip: IpAddr,

        /// The port to listen on for anonymous inbound connections from Tor Daemon.
        ///
        /// This setting is only took into account if `p2p.tor_net.enabled` is set to "Daemon".
        ///
        /// Type         | Number
        /// Valid values | 0..65534
        /// Examples     | 18080, 9999, 5432
        pub listening_port: u16,
    }

    /// Tor config
    #[derive(Debug, Default, Clone, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields, default)]
    #[allow(rustdoc::broken_intra_doc_links)]
    pub struct TorConfig {

        #[comment_out = true]
        /// Enable Tor network by specifying how to connect to it.
        ///
        /// Setting this to "" (an empty string) will disable Tor.
        ///
        /// When "Daemon" is set, the Tor daemon address to use can be
        /// specified in `tor.daemon_addr`.
        ///
        /// Type         | String
        /// Valid values | "Arti", "Daemon", "Off"
        /// Examples     | "Arti"
        pub mode: TorMode,

        #[child = true]
        /// Arti config
        pub arti: ArtiConfig,

        #[child = true]
        /// Tor Daemon config
        pub daemon: TorDaemonConfig,
    }
}

impl Default for TorDaemonConfig {
    fn default() -> Self {
        Self {
            address: "127.0.0.1:9050".parse().unwrap(),
            anonymous_inbound: String::new(),
            listening_ip: Ipv4Addr::LOCALHOST.into(),
            listening_port: 18083,
        }
    }
}

impl Default for ArtiConfig {
    fn default() -> Self {
        Self {
            directory_path: CUPRATE_DATA_DIR.join("arti"),
            isolated_circuit: false,
            onion_service_pow: false,
        }
    }
}

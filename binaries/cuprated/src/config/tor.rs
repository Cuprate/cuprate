use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use cuprate_helper::fs::CUPRATE_DATA_DIR;

use crate::config::macros::config_struct;

config_struct! {
    /// Tor config
    #[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields, default)]
    #[allow(rustdoc::broken_intra_doc_links)]
    pub struct TorConfig {
        /// The IP address and port of the external Tor daemon to use for outgoing connections.
        ///
        /// Type     | Socket address
        /// Examples | "[::1]:9050", "127.0.0.1:9050"
        pub daemon_addr: SocketAddr,

        /// The IP address to bind and listen on for anonymous inbound connections from Tor Daemon.
        ///
        /// This setting is only took into account if `p2p.tor_net.enabled` is set to "Daemon".
        ///
        /// Type     | IP address
        /// Examples | "0.0.0.0", "192.168.1.50", "::", "2001:0db8:85a3:0000:0000:8a2e:0370:7334"
        pub daemon_listening_ip: IpAddr,

        /// The port to listen on for anonymous inbound connections from Tor Daemon.
        ///
        /// This setting is only took into account if `p2p.tor_net.enabled` is set to "Daemon".
        ///
        /// Type         | Number
        /// Valid values | 0..65534
        /// Examples     | 18080, 9999, 5432
        pub daemon_listening_port: u16,

        /// Path to the arti state directory.
        ///
        /// Type         | String
        /// Valid values | false, true
        /// Examples     | false
        pub arti_directory_path: PathBuf,

        /// Enable isolated circuits for Arti.
        ///
        /// If set, Arti will use different tor circuits for each connections. This can
        /// cause stability issues if the connection count is important.
        ///
        /// Type         | boolean
        /// Valid values | false, true
        /// Examples     | false
        pub arti_isolated_circuit: bool,

        /// Enable PoW security for Arti.
        ///
        /// If set, Arti will enforce an EquiX PoW to be resolved for
        /// other nodes to complete a rendez-vous request. This is a
        /// DDoS mitigation and is only enabled during heavy load.
        ///
        /// Type         | boolean
        /// Valid values | false, true
        /// Examples     | false
        pub arti_onion_service_pow: bool,
    }
}

impl Default for TorConfig {
    fn default() -> Self {
        Self {
            daemon_addr: "127.0.0.1:9050".parse().unwrap(),
            daemon_listening_ip: Ipv4Addr::LOCALHOST.into(),
            daemon_listening_port: 18083,
            arti_directory_path: CUPRATE_DATA_DIR.join("arti"),
            arti_isolated_circuit: false,
            arti_onion_service_pow: false,
        }
    }
}

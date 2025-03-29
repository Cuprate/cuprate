use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use serde::{Deserialize, Serialize};

/// RPC config.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(deny_unknown_fields, default)]
pub struct RpcConfig {
    /// Socket address for unrestricted RPC.
    pub address: SocketAddr,

    /// Socket address for restricted RPC.
    /// If [`None`], the restricted RPC server will be disabled.
    pub address_restricted: Option<SocketAddr>,
}

impl RpcConfig {
    /// Return the port of the restricted RPC address (if set).
    pub fn port_restricted(&self) -> Option<u16> {
        self.address_restricted.map(|s| s.port())
    }
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            address: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 18081)),
            address_restricted: None,
        }
    }
}

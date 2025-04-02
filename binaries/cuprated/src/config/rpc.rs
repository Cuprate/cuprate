use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

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

    /// Enable request gzip (de)compression if `true`, else, disable.
    pub gzip: bool,

    /// Enable request br (de)compression if `true`, else, disable.
    pub br: bool,

    /// If a restricted request is above this byte limit, it will be rejected.
    pub request_byte_limit: usize,

    /// If a restricted request does not complete
    /// within the specified timeout it will be aborted.
    pub request_timeout: Duration,

    /// Rate limit the amount of restricted requests per minute to this amount.
    ///
    /// TODO: this field does nothing for now.
    pub max_requests_per_minute: u64,

    /// Max amount of TCP sockets that are allowed to be
    /// opened at the same time for restricted RPC.
    ///
    /// TODO: this field does nothing for now.
    pub max_tcp_sockets: u64,
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
            gzip: true,
            br: true,
            request_byte_limit: 1024 * 1024, // 1 megabyte
            request_timeout: Duration::from_secs(60),
            max_requests_per_minute: 600, // 10 per second
            max_tcp_sockets: 512, // 1024 max open files on linux - other files opened by `cuprated` + leeway
        }
    }
}

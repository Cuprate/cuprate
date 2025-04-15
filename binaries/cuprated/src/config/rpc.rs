use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

use serde::{Deserialize, Serialize};

crate::config::macros::config_struct! {
    /// RPC config.
    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[serde(deny_unknown_fields, default)]
    pub struct RpcConfig {
        /// Socket address for unrestricted RPC.
        pub address: SocketAddr,

        /// Socket address for restricted RPC.
        /// If [`None`], the restricted RPC server will be disabled.
        pub address_restricted: Option<SocketAddr>,

        /// Toggle request gzip (de)compression.
        ///
        /// Setting this to `true` will allow the RPC server
        /// to accept gzip compressed requests and send
        /// gzip compressed responses if the client
        /// has `Content-Encoding: gzip` set.
        pub gzip: bool,

        /// Toggle request br (de)compression.
        ///
        /// Setting this to `true` will allow the RPC server
        /// to accept br compressed requests and send
        /// br compressed responses if the client
        /// has `Content-Encoding: br` set.
        pub br: bool,

        /// If a restricted request is above this byte limit, it will be rejected.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 5242880 (5MB), 10485760 (10MB)
        pub restricted_request_byte_limit: usize,

        // TODO: <https://github.com/Cuprate/cuprate/issues/445>.
    }
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
            restricted_request_byte_limit: 1024 * 1024, // 1 megabyte
        }
    }
}

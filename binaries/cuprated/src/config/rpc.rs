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
        /// Address and port for the unrestricted RPC server.
        ///
        /// If this is left empty, the unrestricted
        /// RPC server will be disabled.
        ///
        /// Type     | IPv4/IPv6 address + port
        /// Examples | "", "127.0.0.1:18081", "192.168.1.50:18085"
        pub address: Option<SocketAddr>,

        /// Address and port for the restricted RPC server.
        ///
        /// If this is left empty, the restricted
        /// RPC server will be disabled.
        ///
        /// Type     | IPv4/IPv6 address + port
        /// Examples | "", "0.0.0.0:18089", "192.168.1.50:18089"
        pub address_restricted: Option<SocketAddr>,

        /// Allow the unrestricted RPC server to be public.
        ///
        /// ⚠️ WARNING ⚠️
        /// -------------
        /// Unrestricted RPC should almost never be made available
        /// to the wider internet. If `address` is a non-local
        /// address, `cuprated` will crash - unless this setting
        /// is set to `true`.
        ///
        /// Type         | boolean
        /// Valid values | true, false
        pub i_know_what_im_doing_allow_public_unrestricted_rpc: bool,

        /// Toggle request gzip (de)compression.
        ///
        /// Setting this to `true` will allow the RPC server
        /// to accept gzip compressed requests and send
        /// gzip compressed responses if the client
        /// has `Content-Encoding: gzip` set.
        ///
        /// Type         | boolean
        /// Valid values | true, false
        pub gzip: bool,

        /// Toggle request br (de)compression.
        ///
        /// Setting this to `true` will allow the RPC server
        /// to accept br compressed requests and send
        /// br compressed responses if the client
        /// has `Content-Encoding: br` set.
        ///
        /// Type         | boolean
        /// Valid values | true, false
        pub br: bool,

        /// If a restricted request is above this byte limit, it will be rejected.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 5242880 (5MB), 10485760 (10MB)
        pub restricted_request_byte_limit: usize,

        // TODO: <https://github.com/Cuprate/cuprate/issues/445>
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
            address: Some(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::LOCALHOST,
                18081,
            ))),
            address_restricted: None,
            i_know_what_im_doing_allow_public_unrestricted_rpc: false,
            gzip: true,
            br: true,
            restricted_request_byte_limit: 1024 * 1024, // 1 megabyte
        }
    }
}

use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::config::macros::config_struct;

config_struct! {
    /// RPC config.
    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[serde(deny_unknown_fields, default)]
    pub struct RpcConfig {
        #[child = true]
        /// Configuration for the restricted RPC server.
        pub unrestricted: SharedRpcConfig,

        #[child = true]
        /// Configuration for the restricted RPC server.
        pub restricted: SharedRpcConfig,

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
}

impl RpcConfig {
    /// Return the port of the restricted RPC address (if set).
    pub fn port_restricted(&self) -> Option<u16> {
        self.restricted.address.map(|s| s.port())
    }
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            unrestricted: SharedRpcConfig {
                address: Some(SocketAddr::V4(SocketAddrV4::new(
                    Ipv4Addr::LOCALHOST,
                    18081,
                ))),
                request_byte_limit: 0,
                ..Default::default()
            },
            restricted: Default::default(),
            i_know_what_im_doing_allow_public_unrestricted_rpc: false,
        }
    }
}

config_struct! {
    /// Shared RPC configuration options.
    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[serde(deny_unknown_fields, default)]
    pub struct SharedRpcConfig {
        #[comment_out = true]
        /// Address and port for the RPC server.
        ///
        /// If this is left empty, the server will be disabled.
        ///
        /// Type     | IPv4/IPv6 address + port
        /// Examples | "", "127.0.0.1:18081", "192.168.1.50:18085"
        pub address: Option<SocketAddr>,

        #[comment_out = true]
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

        #[comment_out = true]
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

        #[comment_out = true]
        /// If a request is above this byte limit, it will be rejected.
        ///
        /// Setting this to `0` will disable the limit.
        ///
        /// Type         | Number
        /// Valid values | >= 0
        /// Examples     | 0 (no limit), 5242880 (5MB), 10485760 (10MB)
        pub request_byte_limit: usize,

        // TODO: <https://github.com/Cuprate/cuprate/issues/445>
    }
}

impl Default for SharedRpcConfig {
    fn default() -> Self {
        Self {
            address: None,
            gzip: true,
            br: true,
            // 1 megabyte.
            // <https://github.com/monero-project/monero/blob/3b01c490953fe92f3c6628fa31d280a4f0490d28/src/cryptonote_config.h#L134>
            request_byte_limit: 1024 * 1024,
        }
    }
}

use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::config::macros::config_struct;

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

impl RpcConfig {
    /// Return the restricted RPC port for P2P if available and public.
    pub const fn port_for_p2p(&self) -> u16 {
        // TODO: implement `--public-node`.
        let public_node = false;

        let addr = &self.restricted.shared.address;

        if public_node
            && self.restricted.shared.enable
            && cuprate_helper::net::ip_is_local(addr.ip())
        {
            addr.port()
        } else {
            0
        }
    }
}

config_struct! {
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

        #[flatten = true]
        /// Shared config.
        ##[serde(flatten)]
        pub shared: SharedRpcConfig,
    }
}

impl Default for UnrestrictedRpcConfig {
    fn default() -> Self {
        Self {
            i_know_what_im_doing_allow_public_unrestricted_rpc: false,
            shared: SharedRpcConfig {
                address: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 18081)),
                enable: true,
                gzip: true,
                br: true,
                request_byte_limit: 0,
            },
        }
    }
}

config_struct! {
    #[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields, default)]
    pub struct RestrictedRpcConfig {
        #[flatten = true]
        /// Shared config.
        ##[serde(flatten)]
        pub shared: SharedRpcConfig,
    }
}

config_struct! {
    /// Shared RPC configuration options.
    ///
    /// Both RPC servers use these values.
    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
    #[serde(deny_unknown_fields, default)]
    pub struct SharedRpcConfig {
        /// The address and port the RPC server will listen on.
        ///
        /// Type     | IPv4/IPv6 address + port
        /// Examples | "", "127.0.0.1:18081", "192.168.1.50:18085"
        pub address: SocketAddr,

        /// Toggle the RPC server.
        ///
        /// If `true` the RPC server will be enable.
        /// If `false` the RPC server will be disabled.
        ///
        /// Type     | boolean
        /// Examples | true, false
        pub enable: bool,

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
    /// This returns the default for [`RestrictedRpcConfig`].
    fn default() -> Self {
        Self {
            address: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 18089)),
            enable: false,
            gzip: true,
            br: true,
            // 1 megabyte.
            // <https://github.com/monero-project/monero/blob/3b01c490953fe92f3c6628fa31d280a4f0490d28/src/cryptonote_config.h#L134>
            request_byte_limit: 1024 * 1024,
        }
    }
}

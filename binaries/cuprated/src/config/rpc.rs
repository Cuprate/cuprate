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

        /// Advertise the restricted RPC port.
        ///
        /// Setting this to `true` will make `cuprated`
        /// share the restricted RPC server's port
        /// publically to the P2P network.
        ///
        /// Type         | boolean
        /// Valid values | true, false
        pub advertise: bool,
    }
}

impl RestrictedRpcConfig {
    /// Return the restricted RPC port for P2P if available and public.
    pub const fn port_for_p2p(&self) -> u16 {
        if self.advertise && self.shared.enable {
            self.shared.address.port()
        } else {
            0
        }
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
    /// This returns the default for [`RestrictedRpcConfig`].
    fn default() -> Self {
        Self {
            address: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 18089)),
            enable: false,
            // 1 megabyte.
            // <https://github.com/monero-project/monero/blob/3b01c490953fe92f3c6628fa31d280a4f0490d28/src/cryptonote_config.h#L134>
            request_byte_limit: 1024 * 1024,
        }
    }
}

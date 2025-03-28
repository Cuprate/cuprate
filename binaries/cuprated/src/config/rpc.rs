use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    path::Path,
    time::Duration,
};

use serde::{Deserialize, Serialize};

/// RPC config.
#[derive(
    Copy, Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(deny_unknown_fields, default)]
pub struct RpcConfig {
    asdf: usize,
}

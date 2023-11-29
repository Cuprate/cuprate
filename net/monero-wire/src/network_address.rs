// Rust Levin Library
// Written in 2023 by
//   Cuprate Contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//

//! This module defines the addresses that will get passed around the
//! Monero network. Core Monero has 4 main addresses: IPv4, IPv6, Tor,
//! I2p. Currently this module only has IPv(4/6).
//!
use std::{hash::Hash, net, net::SocketAddr};

use serde::{Deserialize, Serialize};

mod serde_helper;
use serde_helper::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NetZone {
    Public,
    Tor,
    I2p,
}

/// A network address which can be encoded into the format required
/// to send to other Monero peers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "TaggedNetworkAddress")]
#[serde(into = "TaggedNetworkAddress")]
pub enum NetworkAddress {
    Clear(SocketAddr),
}

impl NetworkAddress {
    pub fn get_zone(&self) -> NetZone {
        match self {
            NetworkAddress::Clear(_) => NetZone::Public,
        }
    }

    pub fn is_loopback(&self) -> bool {
        // TODO
        false
    }

    pub fn is_local(&self) -> bool {
        // TODO
        false
    }

    pub fn port(&self) -> u16 {
        match self {
            NetworkAddress::Clear(ip) => ip.port(),
        }
    }
}

impl From<net::SocketAddrV4> for NetworkAddress {
    fn from(value: net::SocketAddrV4) -> Self {
        NetworkAddress::Clear(value.into())
    }
}

impl From<net::SocketAddrV6> for NetworkAddress {
    fn from(value: net::SocketAddrV6) -> Self {
        NetworkAddress::Clear(value.into())
    }
}

impl From<SocketAddr> for NetworkAddress {
    fn from(value: SocketAddr) -> Self {
        match value {
            SocketAddr::V4(v4) => v4.into(),
            SocketAddr::V6(v6) => v6.into(),
        }
    }
}

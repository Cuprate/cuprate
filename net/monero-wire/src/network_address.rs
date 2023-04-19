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
use std::{hash::Hash, net};

use epee_serde::Value;
use serde::{de, ser::SerializeStruct, Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NetZone {
    Public, 
    Tor,
    I2p
}

/// An IPv4 address with a port
#[derive(Clone, Copy, Serialize, Debug, Default, PartialEq, Eq, Hash)]
pub struct IPv4Address {
    /// IP address
    pub m_ip: u32,
    /// Port
    pub m_port: u16,
}

impl From<net::SocketAddrV4> for IPv4Address {
    fn from(value: net::SocketAddrV4) -> Self {
        IPv4Address {
            m_ip: u32::from_le_bytes(value.ip().octets()),
            m_port: value.port(),
        }
    }
}

impl IPv4Address {
    fn from_value<E: de::Error>(value: &Value) -> Result<Self, E> {
        let m_ip = get_val_from_map!(value, "m_ip", get_u32, "u32");

        let m_port = get_val_from_map!(value, "m_port", get_u16, "u16");

        Ok(IPv4Address {
            m_ip: *m_ip,
            m_port: *m_port,
        })
    }
}

/// An IPv6 address with a port
#[derive(Clone, Copy, Serialize, Debug, Default, PartialEq, Eq, Hash)]
pub struct IPv6Address {
    /// Address
    pub addr: [u8; 16],
    /// Port
    pub m_port: u16,
}

impl From<net::SocketAddrV6> for IPv6Address {
    fn from(value: net::SocketAddrV6) -> Self {
        IPv6Address {
            addr: value.ip().octets(),
            m_port: value.port(),
        }
    }
}

impl IPv6Address {
    fn from_value<E: de::Error>(value: &Value) -> Result<Self, E> {
        let addr = get_val_from_map!(value, "addr", get_bytes, "Vec<u8>");

        let m_port = get_val_from_map!(value, "m_port", get_u16, "u16");

        Ok(IPv6Address {
            addr: addr
                .clone()
                .try_into()
                .map_err(|_| E::invalid_length(addr.len(), &"a 16-byte array"))?,
            m_port: *m_port,
        })
    }
}

/// A network address which can be encoded into the format required
/// to send to other Monero peers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NetworkAddress {
    /// IPv4
    IPv4(IPv4Address),
    /// IPv6
    IPv6(IPv6Address),
}


impl NetworkAddress {
    pub fn get_zone(&self) -> NetZone {
        match self {
            NetworkAddress::IPv4(_) | NetworkAddress::IPv6(_) => NetZone::Public,
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
            NetworkAddress::IPv4(ip) => ip.m_port,
            NetworkAddress::IPv6(ip) => ip.m_port,
        }
    }
}

impl Default for NetworkAddress {
    fn default() -> Self {
        Self::IPv4(IPv4Address::default())
    }
}

impl From<net::SocketAddrV4> for NetworkAddress {
    fn from(value: net::SocketAddrV4) -> Self {
        NetworkAddress::IPv4(value.into())
    }
}

impl From<net::SocketAddrV6> for NetworkAddress {
    fn from(value: net::SocketAddrV6) -> Self {
        NetworkAddress::IPv6(value.into())
    }
}

impl From<net::SocketAddr> for NetworkAddress {
    fn from(value: net::SocketAddr) -> Self {
        match value {
            net::SocketAddr::V4(v4) => v4.into(),
            net::SocketAddr::V6(v6) => v6.into(),
        }
    }
}

impl<'de> Deserialize<'de> for NetworkAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let addr_type = get_val_from_map!(value, "type", get_u8, "u8");

        Ok(match addr_type {
            1 => NetworkAddress::IPv4(IPv4Address::from_value(get_field_from_map!(value, "addr"))?),
            2 => NetworkAddress::IPv6(IPv6Address::from_value(get_field_from_map!(value, "addr"))?),
            _ => return Err(de::Error::custom("Network address type currently unsupported")),
        })
    }
}

impl Serialize for NetworkAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("", 2)?;
        match self {
            NetworkAddress::IPv4(v) => {
                state.serialize_field("type", &1_u8)?;
                state.serialize_field("addr", v)?;
            },
            NetworkAddress::IPv6(v) => {
                state.serialize_field("type", &2_u8)?;
                state.serialize_field("addr", v)?;
            },
        }
        state.end()
    }
}

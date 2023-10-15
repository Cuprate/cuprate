use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::NetworkAddress;

#[derive(Serialize, Deserialize)]
pub(crate) struct TaggedNetworkAddress {
    #[serde(rename = "type")]
    ty: u8,
    #[serde(flatten)]
    addr: RawNetworkAddress,
}

#[derive(Error, Debug)]
#[error("Invalid network address tag")]
pub(crate) struct InvalidNetworkAddressTag;

impl TryFrom<TaggedNetworkAddress> for NetworkAddress {
    type Error = InvalidNetworkAddressTag;

    fn try_from(value: TaggedNetworkAddress) -> Result<Self, Self::Error> {
        Ok(match (value.ty, value.addr) {
            (1, RawNetworkAddress::IPv4(addr)) => NetworkAddress::IPv4(addr),
            (2, RawNetworkAddress::IPv6(addr)) => NetworkAddress::IPv6(addr),
            _ => return Err(InvalidNetworkAddressTag),
        })
    }
}

impl From<NetworkAddress> for TaggedNetworkAddress {
    fn from(value: NetworkAddress) -> Self {
        match value {
            NetworkAddress::IPv4(addr) => TaggedNetworkAddress {
                ty: 1,
                addr: RawNetworkAddress::IPv4(addr),
            },
            NetworkAddress::IPv6(addr) => TaggedNetworkAddress {
                ty: 2,
                addr: RawNetworkAddress::IPv6(addr),
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum RawNetworkAddress {
    /// IPv4
    IPv4(#[serde(with = "SocketAddrV4Def")] SocketAddrV4),
    /// IPv6
    IPv6(#[serde(with = "SocketAddrV6Def")] SocketAddrV6),
}

#[derive(Deserialize, Serialize)]
#[serde(remote = "SocketAddrV4")]
pub(crate) struct SocketAddrV4Def {
    #[serde(getter = "get_ip_v4")]
    m_ip: u32,
    #[serde(getter = "SocketAddrV4::port")]
    m_port: u16,
}

fn get_ip_v4(addr: &SocketAddrV4) -> u32 {
    u32::from_be_bytes(addr.ip().octets())
}

impl From<SocketAddrV4Def> for SocketAddrV4 {
    fn from(def: SocketAddrV4Def) -> SocketAddrV4 {
        SocketAddrV4::new(Ipv4Addr::from(def.m_ip), def.m_port)
    }
}

#[derive(Deserialize, Serialize)]
#[serde(remote = "SocketAddrV6")]
pub(crate) struct SocketAddrV6Def {
    #[serde(getter = "get_ip_v6")]
    addr: [u8; 16],
    #[serde(getter = "SocketAddrV6::port")]
    m_port: u16,
}

fn get_ip_v6(addr: &SocketAddrV6) -> [u8; 16] {
    addr.ip().octets()
}

impl From<SocketAddrV6Def> for SocketAddrV6 {
    fn from(def: SocketAddrV6Def) -> SocketAddrV6 {
        SocketAddrV6::new(Ipv6Addr::from(def.addr), def.m_port, 0, 0)
    }
}

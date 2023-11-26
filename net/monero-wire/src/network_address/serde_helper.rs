use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::NetworkAddress;

#[derive(Serialize, Deserialize)]
pub(crate) struct TaggedNetworkAddress {
    #[serde(rename = "type")]
    ty: u8,
    addr: AllFieldsNetworkAddress,
}

#[derive(Error, Debug)]
#[error("Invalid network address")]
pub(crate) struct InvalidNetworkAddress;

impl TryFrom<TaggedNetworkAddress> for NetworkAddress {
    type Error = InvalidNetworkAddress;

    fn try_from(value: TaggedNetworkAddress) -> Result<Self, Self::Error> {
        value
            .addr
            .try_into_network_address(value.ty)
            .ok_or(InvalidNetworkAddress)
    }
}

impl From<NetworkAddress> for TaggedNetworkAddress {
    fn from(value: NetworkAddress) -> Self {
        match value {
            NetworkAddress::IPv4(addr) => TaggedNetworkAddress {
                ty: 1,
                addr: AllFieldsNetworkAddress {
                    m_ip: Some(u32::from_be_bytes(addr.ip().octets())),
                    m_port: Some(addr.port()),
                    ..Default::default()
                },
            },
            NetworkAddress::IPv6(addr) => TaggedNetworkAddress {
                ty: 2,
                addr: AllFieldsNetworkAddress {
                    addr: Some(addr.ip().octets()),
                    m_port: Some(addr.port()),
                    ..Default::default()
                },
            },
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
struct AllFieldsNetworkAddress {
    #[serde(skip_serializing_if = "Option::is_none")]
    m_ip: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    m_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    addr: Option<[u8; 16]>,
}

impl AllFieldsNetworkAddress {
    fn try_into_network_address(self, ty: u8) -> Option<NetworkAddress> {
        Some(match ty {
            1 => NetworkAddress::IPv4(SocketAddrV4::new(Ipv4Addr::from(self.m_ip?), self.m_port?)),
            2 => NetworkAddress::IPv6(SocketAddrV6::new(
                Ipv6Addr::from(self.addr?),
                self.m_port?,
                0,
                0,
            )),
            _ => return None,
        })
    }
}

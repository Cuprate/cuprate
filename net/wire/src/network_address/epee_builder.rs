//! Address epee serialization
//!
//! Addresses needs to be serialized into a specific format before being sent to other peers.
//! This module is handling this particular construction.
//!

//---------------------------------------------------------------------------------------------------- Imports

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

use bytes::Buf;
use thiserror::Error;

use cuprate_epee_encoding::{epee_object, EpeeObjectBuilder};
use cuprate_types::AddressType;

use crate::NetworkAddress;

use super::OnionAddr;

//---------------------------------------------------------------------------------------------------- Network address construction

#[derive(Default)]
/// There are no ordering guarantees in epee format and as such all potential fields can be collected during deserialization.
/// The [`AllFieldsNetworkAddress`] is containing, as its name suggest, all optional field describing an address , as if it
/// could be of any type.
struct AllFieldsNetworkAddress {
    /// IPv4 address
    m_ip: Option<u32>,
    /// IP port field
    m_port: Option<u16>,

    /// IPv6 address
    addr: Option<[u8; 16]>,

    /// Alternative network domain name (<domain>.onion or <domain>.i2p)
    host: Option<String>,
    /// Alternative network virtual port
    port: Option<u16>,
}

epee_object!(
    AllFieldsNetworkAddress,
    m_ip: Option<u32>,
    m_port: Option<u16>,
    addr: Option<[u8; 16]>,
    host: Option<String>,
    port: Option<u16>,
);

impl AllFieldsNetworkAddress {
    fn try_into_network_address(self, ty: AddressType) -> Option<NetworkAddress> {
        Some(match ty {
            AddressType::Ipv4 => NetworkAddress::from(SocketAddrV4::new(
                Ipv4Addr::from(self.m_ip?.to_le_bytes()),
                self.m_port?,
            )),
            AddressType::Ipv6 => NetworkAddress::from(SocketAddrV6::new(
                Ipv6Addr::from(self.addr?),
                self.m_port?,
                0,
                0,
            )),
            AddressType::Tor => {
                NetworkAddress::from(OnionAddr::new(self.host?.as_str(), self.port?).ok()?)
            }
            // Invalid
            _ => return None,
        })
    }
}

#[derive(Default)]
/// A serialized network address being communicated to or from a peer.
pub struct TaggedNetworkAddress {
    /// Type of the network address (used later for conversion)
    ty: Option<AddressType>,
    /// All possible fields for a network address
    addr: Option<AllFieldsNetworkAddress>,
}

epee_object!(
    TaggedNetworkAddress,
    ty("type"): Option<AddressType>,
    addr: Option<AllFieldsNetworkAddress>,
);

impl EpeeObjectBuilder<NetworkAddress> for TaggedNetworkAddress {
    fn add_field<B: Buf>(&mut self, name: &str, b: &mut B) -> cuprate_epee_encoding::Result<bool> {
        match name {
            "type" => {
                if std::mem::replace(
                    &mut self.ty,
                    Some(cuprate_epee_encoding::read_epee_value(b)?),
                )
                .is_some()
                {
                    return Err(cuprate_epee_encoding::Error::Format(
                        "Duplicate field in data.",
                    ));
                }
                Ok(true)
            }
            "addr" => {
                if std::mem::replace(&mut self.addr, cuprate_epee_encoding::read_epee_value(b)?)
                    .is_some()
                {
                    return Err(cuprate_epee_encoding::Error::Format(
                        "Duplicate field in data.",
                    ));
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn finish(self) -> cuprate_epee_encoding::Result<NetworkAddress> {
        self.try_into()
            .map_err(|_| cuprate_epee_encoding::Error::Value("Invalid network address".to_string()))
    }
}

#[derive(Error, Debug)]
#[error("Invalid network address")]
pub struct InvalidNetworkAddress;

impl TryFrom<TaggedNetworkAddress> for NetworkAddress {
    type Error = InvalidNetworkAddress;

    fn try_from(value: TaggedNetworkAddress) -> Result<Self, Self::Error> {
        value
            .addr
            .ok_or(InvalidNetworkAddress)?
            .try_into_network_address(value.ty.ok_or(InvalidNetworkAddress)?)
            .ok_or(InvalidNetworkAddress)
    }
}

impl From<NetworkAddress> for TaggedNetworkAddress {
    fn from(value: NetworkAddress) -> Self {
        match value {
            NetworkAddress::Clear(addr) => match addr {
                SocketAddr::V4(addr) => Self {
                    ty: Some(AddressType::Ipv4),
                    addr: Some(AllFieldsNetworkAddress {
                        m_ip: Some(u32::from_le_bytes(addr.ip().octets())),
                        m_port: Some(addr.port()),
                        addr: None,
                        host: None,
                        port: None,
                    }),
                },
                SocketAddr::V6(addr) => Self {
                    ty: Some(AddressType::Ipv6),
                    addr: Some(AllFieldsNetworkAddress {
                        addr: Some(addr.ip().octets()),
                        m_port: Some(addr.port()),
                        m_ip: None,
                        host: None,
                        port: None,
                    }),
                },
            },
            NetworkAddress::Tor(onion_addr) => Self {
                ty: Some(AddressType::Tor),
                addr: Some(AllFieldsNetworkAddress {
                    m_ip: None,
                    m_port: None,
                    addr: None,
                    host: Some(onion_addr.addr_string()),
                    port: Some(onion_addr.port()),
                }),
            },
        }
    }
}

use bytes::Buf;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

use cuprate_epee_encoding::{epee_object, EpeeObjectBuilder};
use thiserror::Error;

use crate::NetworkAddress;

#[derive(Default)]
pub struct TaggedNetworkAddress {
    ty: Option<u8>,
    addr: Option<AllFieldsNetworkAddress>,
}

epee_object!(
    TaggedNetworkAddress,
    ty("type"): Option<u8>,
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
                SocketAddr::V4(addr) => TaggedNetworkAddress {
                    ty: Some(1),
                    addr: Some(AllFieldsNetworkAddress {
                        m_ip: Some(u32::from_be_bytes(addr.ip().octets())),
                        m_port: Some(addr.port()),
                        addr: None,
                    }),
                },
                SocketAddr::V6(addr) => TaggedNetworkAddress {
                    ty: Some(2),
                    addr: Some(AllFieldsNetworkAddress {
                        addr: Some(addr.ip().octets()),
                        m_port: Some(addr.port()),
                        m_ip: None,
                    }),
                },
            },
        }
    }
}

#[derive(Default)]
struct AllFieldsNetworkAddress {
    m_ip: Option<u32>,
    m_port: Option<u16>,
    addr: Option<[u8; 16]>,
}

epee_object!(
    AllFieldsNetworkAddress,
    m_ip: Option<u32>,
    m_port: Option<u16>,
    addr: Option<[u8; 16]>,
);

impl AllFieldsNetworkAddress {
    fn try_into_network_address(self, ty: u8) -> Option<NetworkAddress> {
        Some(match ty {
            1 => NetworkAddress::from(SocketAddrV4::new(Ipv4Addr::from(self.m_ip?), self.m_port?)),
            2 => NetworkAddress::from(SocketAddrV6::new(
                Ipv6Addr::from(self.addr?),
                self.m_port?,
                0,
                0,
            )),
            _ => return None,
        })
    }
}

use std::{hash::Hash, net};

use epee_serde::Value;
use serde::{de, ser::SerializeStruct, Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Debug, PartialEq, Eq, Hash)]
pub struct IPv4Address {

    pub m_ip: u32,
    pub m_port: u16,
}

impl From<net::SocketAddrV4> for IPv4Address {
    fn from(value: net::SocketAddrV4) -> Self {
        IPv4Address {
            m_ip: u32::from_le_bytes(value.ip().octets()), 
            m_port: value.port() 
        }
    }
}

impl IPv4Address {
    pub fn from_value<E: de::Error>(value: &Value) -> Result<Self, E> {
        let m_ip = get_val_from_map!(value, "m_ip", get_u32, "u32");

        let m_port = get_val_from_map!(value, "m_port", get_u16, "u16");

        Ok(IPv4Address {
            m_ip: *m_ip,
            m_port: *m_port,
        })
    }
}

#[derive(Clone, Copy, Serialize, Debug, PartialEq, Eq, Hash)]
pub struct IPv6Address {
    pub addr: [u8; 16],
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
    pub fn from_value<E: de::Error>(value: &Value) -> Result<Self, E> {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NetworkAddress {
    IPv4(IPv4Address),
    IPv6(IPv6Address),
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
            _ => {
                return Err(de::Error::custom(
                    "Network address type currently unsupported",
                ))
            }
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
            }
            NetworkAddress::IPv6(v) => {
                state.serialize_field("type", &2_u8)?;
                state.serialize_field("addr", v)?;
            }
        }
        state.end()
    }
}

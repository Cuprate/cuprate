use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};

use epee_encoding::{
    error::Error,
    io::{Read, Write},
    read_epee_value, write_field, EpeeObject, EpeeObjectBuilder, EpeeValue,
};

use super::NetworkAddress;

impl EpeeObject for NetworkAddress {
    type Builder = NetworkAddressBuilder;

    fn number_of_fields(&self) -> u64 {
        2
    }

    fn write_fields<W: Write>(&self, w: &mut W) -> epee_encoding::error::Result<()> {
        match self {
            NetworkAddress::IPv4(ip) => {
                write_field(&1_u8, "type", w)?;
                let addr = NetworkAddressWriter {
                    host: ("m_ip", &u32::from_be_bytes(ip.ip().octets())),
                    port: ("m_port", ip.port()),
                };
                write_field(&addr, "addr", w)
            }
            NetworkAddress::IPv6(ip) => {
                write_field(&2_u8, "type", w)?;
                let addr = NetworkAddressWriter {
                    host: ("addr", &ip.ip().octets()),
                    port: ("m_port", ip.port()),
                };
                write_field(&addr, "addr", w)
            }
        }
    }
}

struct NetworkAddressWriter<'a, T> {
    host: (&'static str, &'a T),
    port: (&'static str, u16),
}

#[derive(Default)]
struct NetworkAddressWBuilder;

impl<'a, T> EpeeObjectBuilder<NetworkAddressWriter<'a, T>> for NetworkAddressWBuilder {
    fn add_field<R: Read>(&mut self, name: &str, r: &mut R) -> epee_encoding::Result<bool> {
        panic!("Not used")
    }

    fn finish(self) -> epee_encoding::Result<NetworkAddressWriter<'a, T>> {
        panic!("Not used")
    }
}

impl<'a, T: EpeeValue> EpeeObject for NetworkAddressWriter<'a, T> {
    type Builder = NetworkAddressWBuilder;

    fn number_of_fields(&self) -> u64 {
        2
    }

    fn write_fields<W: Write>(&self, w: &mut W) -> epee_encoding::Result<()> {
        write_field(self.host.1, self.host.0, w)?;
        write_field(&self.port.1, self.port.0, w)
    }
}
#[derive(Default)]
struct NetworkAddressBuilderIntermediate {
    m_ip: Option<u32>,
    addr: Option<[u8; 16]>,
    m_port: Option<u16>,
    port: Option<u16>,
    host_tor: Option<[u8; 63]>,
    host_i2p: Option<[u8; 61]>,
}

impl EpeeObject for NetworkAddressBuilderIntermediate {
    type Builder = Self;

    fn number_of_fields(&self) -> u64 {
        panic!("This is only used on deserialization")
    }

    fn write_fields<W: Write>(&self, w: &mut W) -> epee_encoding::error::Result<()> {
        panic!("This is only used on deserialization")
    }
}

impl EpeeObjectBuilder<NetworkAddressBuilderIntermediate> for NetworkAddressBuilderIntermediate {
    fn add_field<R: Read>(&mut self, name: &str, r: &mut R) -> epee_encoding::error::Result<bool> {
        match name {
            "m_ip" => self.m_ip = Some(read_epee_value(r)?),
            "addr" => self.addr = Some(read_epee_value(r)?),
            "m_port" => self.m_port = Some(read_epee_value(r)?),
            "port" => self.port = Some(read_epee_value(r)?),
            "host" => {
                let host: Vec<u8> = read_epee_value(r)?;
                if host.len() == 63 {
                    self.host_tor = Some(host.try_into().unwrap());
                } else if host.len() == 61 {
                    self.host_i2p = Some(host.try_into().unwrap());
                }
            }
            _ => return Ok(false),
        }

        Ok(true)
    }

    fn finish(self) -> epee_encoding::error::Result<NetworkAddressBuilderIntermediate> {
        Ok(self)
    }
}

#[derive(Default)]
pub struct NetworkAddressBuilder {
    ty: Option<u8>,
    addr: Option<NetworkAddressBuilderIntermediate>,
}

impl EpeeObjectBuilder<NetworkAddress> for NetworkAddressBuilder {
    fn add_field<R: Read>(&mut self, name: &str, r: &mut R) -> epee_encoding::error::Result<bool> {
        match name {
            "type" => self.ty = Some(read_epee_value(r)?),
            "addr" => self.addr = Some(read_epee_value(r)?),
            _ => return Ok(false),
        }

        Ok(true)
    }
    fn finish(self) -> epee_encoding::error::Result<NetworkAddress> {
        let addr = self
            .addr
            .ok_or(Error::Format("Required field was not in data"))?;

        Ok(
            match self
                .ty
                .ok_or(Error::Format("Required field was not in data"))?
            {
                1 => NetworkAddress::IPv4(SocketAddrV4::new(
                    Ipv4Addr::from(
                        addr.m_ip
                            .ok_or(Error::Format("Required field was not in data"))?,
                    ),
                    addr.m_port
                        .ok_or(Error::Format("Required field was not in data"))?,
                )),
                2 => NetworkAddress::IPv6(SocketAddrV6::new(
                    Ipv6Addr::from(
                        addr.addr
                            .ok_or(Error::Format("Required field was not in data"))?,
                    ),
                    addr.m_port
                        .ok_or(Error::Format("Required field was not in data"))?,
                    0,
                    0,
                )),
                // TODO:  tor/ i2p addresses
                _ => return Err(Error::Value("Unsupported network address")),
            },
        )
    }
}

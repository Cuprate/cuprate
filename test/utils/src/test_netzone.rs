//! Test net zone.
//!
//! This module contains a test network zone, this network zone use channels as the network layer to simulate p2p
//! communication.
//!
use std::{
    fmt::Formatter,
    net::{Ipv4Addr, SocketAddr},
};

use borsh::{BorshDeserialize, BorshSerialize};

use cuprate_p2p_core::{NetZoneAddress, NetworkZone};
use cuprate_wire::network_address::{NetworkAddress, NetworkAddressIncorrectZone};

/// An address on the test network
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct TestNetZoneAddr(pub u32);

impl NetZoneAddress for TestNetZoneAddr {
    type BanID = Self;

    fn set_port(&mut self, _: u16) {}

    fn make_canonical(&mut self) {}

    fn ban_id(&self) -> Self::BanID {
        *self
    }

    fn should_add_to_peer_list(&self) -> bool {
        true
    }
}

impl std::fmt::Display for TestNetZoneAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("test client, id: {}", self.0).as_str())
    }
}

impl From<TestNetZoneAddr> for NetworkAddress {
    fn from(value: TestNetZoneAddr) -> Self {
        Self::Clear(SocketAddr::new(Ipv4Addr::from(value.0).into(), 18080))
    }
}

impl TryFrom<NetworkAddress> for TestNetZoneAddr {
    type Error = NetworkAddressIncorrectZone;

    fn try_from(value: NetworkAddress) -> Result<Self, Self::Error> {
        match value {
            NetworkAddress::Clear(soc) => match soc {
                SocketAddr::V4(v4) => Ok(Self(u32::from_be_bytes(v4.ip().octets()))),
                SocketAddr::V6(_) => panic!("None v4 address in test code"),
            },
        }
    }
}

/// TODO
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TestNetZone<const CHECK_NODE_ID: bool>;

#[async_trait::async_trait]
impl<const CHECK_NODE_ID: bool> NetworkZone for TestNetZone<CHECK_NODE_ID> {
    const NAME: &'static str = "Testing";
    const CHECK_NODE_ID: bool = CHECK_NODE_ID;
    const BROADCAST_OWN_ADDR: bool = false;

    type Addr = TestNetZoneAddr;
}

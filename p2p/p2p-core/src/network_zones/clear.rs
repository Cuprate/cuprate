use std::net::{IpAddr, SocketAddr};

use crate::{NetZoneAddress, NetworkZone};

impl NetZoneAddress for SocketAddr {
    type BanID = IpAddr;

    fn set_port(&mut self, port: u16) {
        Self::set_port(self, port);
    }

    fn ban_id(&self) -> Self::BanID {
        self.ip()
    }

    fn make_canonical(&mut self) {
        let ip = self.ip().to_canonical();
        self.set_ip(ip);
    }

    fn should_add_to_peer_list(&self) -> bool {
        // TODO
        true
    }

    fn as_log(&self) -> impl std::fmt::Display + '_ {
        safelog::Redactable::redacted(self)
    }

    fn to_addr_string(&self) -> String {
        self.to_string()
    }
}

#[derive(Clone, Copy)]
pub enum ClearNet {}

impl NetworkZone for ClearNet {
    const NAME: &'static str = "ClearNet";

    const CHECK_NODE_ID: bool = true;

    const BROADCAST_OWN_ADDR: bool = false;

    type Addr = SocketAddr;
}

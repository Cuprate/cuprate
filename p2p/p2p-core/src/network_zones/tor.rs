//! Tor Zone
//!
//! This module define the Tor Zone that uses the Tor network and .onion service addressing.
//!
//! ### Anonymity
//!
//! This is an anonymous network and is therefore operating under the following behavior:
//! - The node address is blend into its own address book.
//! - This network is only use for relaying transactions.
//!
//! ### Addressing
//!
//! The Tor Zone is using [`OnionAddr`] as its address type.
//!

use cuprate_wire::network_address::OnionAddr;

use crate::{NetZoneAddress, NetworkZone};

impl NetZoneAddress for OnionAddr {
    type BanID = [u8; 56];

    fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    fn ban_id(&self) -> Self::BanID {
        self.domain()
    }

    fn make_canonical(&mut self) {
        // There are no canonical form of an onion address...
    }

    fn should_add_to_peer_list(&self) -> bool {
        // Validation of the onion address has been done at the type construction...
        true
    }
}

#[derive(Clone, Copy)]
pub struct Tor;

impl NetworkZone for Tor {
    const NAME: &'static str = "Tor";

    const CHECK_NODE_ID: bool = false;

    const BROADCAST_OWN_ADDR: bool = true;

    type Addr = OnionAddr;
}

use std::net::SocketAddr;

use cuprate_p2p_core::{client::InternalPeerID, ClearNet, NetworkZone, Tor};
use cuprate_wire::OnionAddr;

/// An identifier for a P2P peer on any network.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CrossNetworkInternalPeerId {
    /// A clear-net peer.
    ClearNet(InternalPeerID<<ClearNet as NetworkZone>::Addr>),
    /// A Tor onion peer.
    Tor(InternalPeerID<<Tor as NetworkZone>::Addr>),
}

impl From<InternalPeerID<SocketAddr>> for CrossNetworkInternalPeerId {
    fn from(addr: InternalPeerID<SocketAddr>) -> Self {
        Self::ClearNet(addr)
    }
}

impl From<InternalPeerID<OnionAddr>> for CrossNetworkInternalPeerId {
    fn from(addr: InternalPeerID<OnionAddr>) -> Self {
        Self::Tor(addr)
    }
}

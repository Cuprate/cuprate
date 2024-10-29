use std::net::SocketAddr;

use cuprate_p2p_core::{client::InternalPeerID, ClearNet, NetworkZone};

/// An identifier for a P2P peer on any network.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CrossNetworkInternalPeerId {
    /// A clear-net peer.
    ClearNet(InternalPeerID<<ClearNet as NetworkZone>::Addr>),
}

impl From<InternalPeerID<<ClearNet as NetworkZone>::Addr>> for CrossNetworkInternalPeerId {
    fn from(addr: InternalPeerID<<ClearNet as NetworkZone>::Addr>) -> Self {
        Self::ClearNet(addr)
    }
}

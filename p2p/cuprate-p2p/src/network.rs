use tokio::sync::broadcast;

use monero_p2p::{protocol::PeerBroadcast, NetworkZone};

use crate::peer_set::LockedPeerSet;

/// The Monero P2P network endpoint.
///
/// This is what is returned to the rest of Cuprate, so they can interact with the network.  
pub struct P2PNetwork<N: NetworkZone> {
    /// The peer-set
    peers: LockedPeerSet<N>,

    /// A channel to broadcast messages to all outbound peers.
    outbound_broadcast: broadcast::Sender<PeerBroadcast>,
    /// A channel to broadcast messages to all inbound peers.
    inbound_broadcast: broadcast::Sender<PeerBroadcast>,
}

use monero_p2p::NetworkZone;

use crate::peer_set::LockedPeerSet;

/// The Monero P2P network endpoint.
///
/// This is what is returned to the rest of Cuprate, so they can interact with the network.  
pub struct P2PNetwork<N: NetworkZone> {
    /// The peer-set
    peers: LockedPeerSet<N>,
}

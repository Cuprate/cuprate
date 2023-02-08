use std::collections::{HashSet};

use cuprate_net::{NetworkAddress, messages::PeerListEntryBase};


#[derive(Debug, Clone, Copy)]
pub struct NetworkTypeSupport {
    pub ipv4: bool,
    pub ipv6: bool,
    pub tor: bool,
    pub i2p: bool
}

impl NetworkTypeSupport {
    pub fn is_peer_supported(&self, peer: &NetworkAddress) -> bool {
        match peer {
            NetworkAddress::IPv4(_) => self.ipv4,
            NetworkAddress::IPv6(_) => self.ipv6,
        }
    } 
}

impl Default for NetworkTypeSupport {
    fn default() -> Self {
        NetworkTypeSupport {
            ipv4: true,
            ipv6: true,
            tor: false,
            i2p: false
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PeerStoreConfig {
    pub white_list_size: usize,
    pub grey_list_size: usize,

    pub network_type_support: NetworkTypeSupport,

    // ban times

}

impl Default for PeerStoreConfig {
    fn default() -> Self {
        PeerStoreConfig {
            white_list_size: 1000,
            grey_list_size: 5000,
            network_type_support: NetworkTypeSupport::default()
        }
    }
}

pub trait PeerStore {

    fn read_white_peers(&self) -> Vec<PeerListEntryBase>;
    fn read_grey_peers(&self) -> Vec<PeerListEntryBase>;
    fn read_anchor_peers(&self) -> Vec<PeerListEntryBase>;

    fn write_white_peers(&mut self, peers: Vec<PeerListEntryBase>);
    fn write_grey_peers(&mut self, peers: Vec<PeerListEntryBase>);
    fn write_anchor_peers(&mut self, peers: Vec<PeerListEntryBase>);

    fn flush(&mut self);
}

pub struct PeerList<S> {
    white: HashSet<PeerListEntryBase>,
    grey: HashSet<PeerListEntryBase>,
    anchor: HashSet<PeerListEntryBase>,

    attempted: HashSet<NetworkAddress>,
    bans: HashSet<NetworkAddress>,

    pruning_seeds: Vec<u32>,

    config: PeerStoreConfig,

    rng: fastrand::Rng,
    store: S,
}

impl<S: PeerStore> PeerList<S> {
    pub fn new(store: S, config: PeerStoreConfig, rng: fastrand::Rng) -> Self {
        let white = HashSet::from_iter(store.read_white_peers());
        let grey = HashSet::from_iter(store.read_grey_peers());
        let anchor = HashSet::from_iter(store.read_anchor_peers());

        Self { 
            white, 
            grey, 
            anchor, 
            bans: HashSet::new(),
            attempted: HashSet::new(),
            pruning_seeds: vec![384, 385, 386, 387, 388, 389, 390, 391],
            config,
            rng,
            store 
        }

    }

    fn is_peer_supported(&self, addr: &NetworkAddress) -> bool {
        self.config.network_type_support.is_peer_supported(&addr)
    }

    pub fn is_peer_baned(&self, addr: &NetworkAddress) -> bool {
        self.bans.contains(addr)
    }

    fn select_optimal_peer(&self, peers: Vec<&PeerListEntryBase>, ) -> Option<PeerListEntryBase> {
        for peer in peers {
            if self.attempted.contains(&peer.adr) {
                continue;
            }
            
            if self.bans.contains(&peer.adr) {
                continue;
            }
    
            if !self.is_peer_supported(&peer.adr) {
                continue;
            }

            // TODO: Check pruning seeds and select a peer with a seed we need

            return Some(*peer)
    
            
        }

        None

    }

    pub fn get_grey_peer(&self) -> Option<PeerListEntryBase> {
        let mut peers = self.grey.iter().collect::<Vec<&PeerListEntryBase>>();
        self.rng.shuffle(&mut peers);
        self.select_optimal_peer(peers)
    }

    pub fn get_white_peer(&self) -> Option<PeerListEntryBase> {
        let mut peers = self.white.iter().collect::<Vec<&PeerListEntryBase>>();
        self.rng.shuffle(&mut peers);
        self.select_optimal_peer(peers)
    }

    pub fn get_anchor_peer(&self) -> Option<PeerListEntryBase> {
        let mut peers = self.anchor.iter().collect::<Vec<&PeerListEntryBase>>();
        self.rng.shuffle(&mut peers);
        self.select_optimal_peer(peers)
    }

    pub fn get_peers_for_handshake(&self) -> Vec<PeerListEntryBase> {
        let mut peers = self.white.iter().map(|p| p.clone()).collect::<Vec<PeerListEntryBase>>();
        self.rng.shuffle(&mut peers);
        if peers.len() > 250 {
            peers[0..250].to_vec()
        } else {
            peers
        }

    }

    pub fn clear_anchor_peers(&mut self) {
        self.anchor.clear()
    }

    pub fn clear_white_peers(&mut self) {
        self.white.clear()
    }

    pub fn clear_grey_peers(&mut self) {
        self.grey.clear()
    }

    fn seen_peer(&mut self, peer: &PeerListEntryBase) -> bool {
        self.bans.contains(&peer.adr) ||
        self.attempted.contains(&peer.adr) ||
        self.white.contains(peer) || 
        self.grey.contains(peer) ||
        self.anchor.contains(peer)
    }


    pub fn incoming_peer_list(&mut self, mut peers: Vec<PeerListEntryBase>) {
        // assume some sanity checks already completed

        for peer in peers.iter_mut() {
            if self.seen_peer(&peer) {
                continue;
            }

            if !self.is_peer_supported(&peer.adr) {
                continue;
            }

            // check pruning seed

            peer.last_seen = 0;

        
            // allow the peer list to grow unbounded here, each `tick` it should be shrunk  
            self.grey.insert(*peer);
        }

    }

    pub fn new_connection(&mut self, peer: PeerListEntryBase) {
        
        self.grey.remove(&peer);

        if self.anchor.contains(&peer) {
            self.anchor.replace(peer);
        } else {
            self.white.replace(peer);
        }
    }

    pub fn update_peer_info(&mut self, peer: PeerListEntryBase) {
        if self.anchor.contains(&peer) {
            self.anchor.replace(peer);
        } else if self.white.contains(&peer) {
            self.white.replace(peer);
        } else {
            self.grey.replace(peer);

        }
    }

    pub fn ban_peer(&mut self, peer: &PeerListEntryBase) {
        self.grey.remove(&peer);
        self.white.remove(&peer);
        self.anchor.remove(&peer);

        self.bans.insert(peer.adr);

    }

}

#[cfg(tests)]
mod tests {
    use crate::types::common::PeerListEntryBase;


    struct TestPeerStore {
        white: Vec<PeerListEntryBase>,
        grey: Vec<PeerListEntryBase>,
        anchor: Vec<PeerListEntryBase>,
    }

    impl TestPeerStore {
        fn init_with_test_data() -> TestPeerStore {

        }
    }

    //fn test_get_
}

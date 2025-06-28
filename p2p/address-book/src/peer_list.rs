use std::collections::{BTreeMap, HashMap, HashSet};

use indexmap::IndexMap;
use rand::Rng;
use cuprate_bucket_set::{BucketMap, Bucketable};
use cuprate_p2p_core::{NetZoneAddress, NetworkZone};
use cuprate_p2p_core::services::ZoneSpecificPeerListEntryBase;
use cuprate_pruning::PruningSeed;
use crate::sealed::BorshNetworkZone;

/// A Peer list in the address book.
///
/// This could either be the white list or gray list.
pub(crate) struct PeerList<Z: BorshNetworkZone> {
    /// The peers with their peer data.
    pub peers: BucketMap<8, ZoneSpecificPeerListEntryBase<Z::Addr>>,
    /// An index of Pruning seed to address, so can quickly grab peers with the blocks
    /// we want.
    ///
    /// Pruning seeds are sorted by first their `log_stripes` and then their stripe.
    /// This means the first peers in this list will store more blocks than peers
    /// later on. So when we need a peer with a certain block we look at the peers
    /// storing more blocks first then work our way to the peers storing less.
    ///
    pruning_seeds: BTreeMap<PruningSeed, Vec<Z::Addr>>,
    /// A hashmap linking `ban_ids` to addresses.
    ban_ids: HashMap<<Z::Addr as NetZoneAddress>::BanID, Vec<Z::Addr>>,
}

impl<Z: BorshNetworkZone> PeerList<Z> {
    /// Creates a new peer list.
    pub(crate) fn new(list: Vec<ZoneSpecificPeerListEntryBase<Z::Addr>>) -> Self {
        let mut peers = BucketMap::new();
        let mut pruning_seeds = BTreeMap::new();
        let mut ban_ids = HashMap::with_capacity(list.len());

        for peer in list {
            let pruning_seed = peer.pruning_seed;
            let adr = peer.adr.clone();
            
            if peers.push(peer).is_none() {
                pruning_seeds
                    .entry(pruning_seed)
                    .or_insert_with(Vec::new)
                    .push(adr.clone());

                ban_ids
                    .entry(adr.ban_id())
                    .or_insert_with(Vec::new)
                    .push(adr);
            }
        }
        Self {
            peers,
            pruning_seeds,
            ban_ids,
        }
    }

    /// Gets the length of the peer list
    pub(crate) fn len(&self) -> usize {
        self.peers.len()
    }

    /// Adds a new peer to the peer list
    pub(crate) fn add_new_peer(&mut self, peer: ZoneSpecificPeerListEntryBase<Z::Addr>) {
        let pruning_seed = peer.pruning_seed;
        let adr = peer.adr.clone();

        if self.peers.push(peer).is_none() {
           self.pruning_seeds
                .entry(pruning_seed)
                .or_insert_with(Vec::new)
                .push(adr.clone());

            self.ban_ids
                .entry(adr.ban_id())
                .or_insert_with(Vec::new)
                .push(adr);
        }
    }

    /// Returns a random peer.
    /// If the pruning seed is specified then we will get a random peer with
    /// that pruning seed otherwise we will just get a random peer in the whole
    /// list.
    ///
    /// The given peer will be removed from the peer list.
    pub(crate) fn take_random_peer<R: Rng>(
        &mut self,
        r: &mut R,
        must_keep_peers: &HashSet<Z::Addr>,
    ) -> Option<ZoneSpecificPeerListEntryBase<Z::Addr>> {
        // Take a random peer and see if it's in the list of must_keep_peers, if it is try again.
        // TODO: improve this
        

        None
    }

    pub(crate) fn get_random_peers<R: Rng>(
        &self,
        r: &mut R,
        len: usize,
    ) -> Vec<ZoneSpecificPeerListEntryBase<Z::Addr>> {
        let mut peers = self.peers.values().copied().choose_multiple(r, len);
        // Order of the returned peers is not random, I am unsure of the impact of this, potentially allowing someone to make guesses about which peers
        // were connected first.
        // So to mitigate this shuffle the result.
        peers.shuffle(r);
        peers.drain(len.min(peers.len())..peers.len());
        peers
    }

    /// Returns a mutable reference to a peer.
    pub(crate) fn get_peer_mut(
        &mut self,
        peer: &Z::Addr,
    ) -> Option<&mut ZoneSpecificPeerListEntryBase<Z::Addr>> {
        self.peers.get_mut(peer)
    }

    /*
    /// Returns true if the list contains this peer.
    pub(crate) fn contains_peer(&self, peer: &Z::Addr) -> bool {
        self.peers.contains_key(peer)
    }
    
     */

    /// Removes a peer from the pruning idx
    ///
    /// MUST NOT BE USED ALONE
    fn remove_peer_pruning_idx(&mut self, peer: &ZoneSpecificPeerListEntryBase<Z::Addr>) {
        remove_peer_idx::<Z>(self.pruning_seeds.get_mut(&peer.pruning_seed), &peer.adr);
        if self
            .pruning_seeds
            .get(&peer.pruning_seed)
            .expect("There must be a peer with this id")
            .is_empty()
        {
            self.pruning_seeds.remove(&peer.pruning_seed);
        }
    }

    /// Removes a peer from the ban idx
    ///
    /// MUST NOT BE USED ALONE
    fn remove_peer_ban_idx(&mut self, peer: &ZoneSpecificPeerListEntryBase<Z::Addr>) {
        remove_peer_idx::<Z>(self.ban_ids.get_mut(&peer.adr.ban_id()), &peer.adr);
        if self
            .ban_ids
            .get(&peer.adr.ban_id())
            .expect("There must be a peer with this id")
            .is_empty()
        {
            self.ban_ids.remove(&peer.adr.ban_id());
        }
    }

    /// Removes a peer from all the indexes
    ///
    /// MUST NOT BE USED ALONE
    fn remove_peer_from_all_idxs(&mut self, peer: &ZoneSpecificPeerListEntryBase<Z::Addr>) {
        self.remove_peer_pruning_idx(peer);
        self.remove_peer_ban_idx(peer);
    }

    /// Removes a peer from the peer list
    pub(crate) fn remove_peer(
        &mut self,
        peer: &Z::Addr,
    ) -> Option<ZoneSpecificPeerListEntryBase<Z::Addr>> {
        let peer_eb = self.peers.remove(peer)?;
        self.remove_peer_from_all_idxs(&peer_eb);
        Some(peer_eb)
    }

    /// Removes all peers with a specific ban id.
    pub(crate) fn remove_peers_with_ban_id(&mut self, ban_id: &<Z::Addr as NetZoneAddress>::BanID) {
        let Some(addresses) = self.ban_ids.get(ban_id) else {
            // No peers to ban
            return;
        };

        for addr in addresses.clone() {
            self.remove_peer(&addr);
        }
    }
}

/// Remove a peer from an index.
fn remove_peer_idx<Z: NetworkZone>(peer_list: Option<&mut Vec<Z::Addr>>, addr: &Z::Addr) {
    if let Some(peer_list) = peer_list {
        if let Some(idx) = peer_list.iter().position(|peer_adr| peer_adr == addr) {
            peer_list.swap_remove(idx);
        } else {
            unreachable!("This function will only be called when the peer exists.");
        }
    } else {
        unreachable!("Index must exist if a peer has that index");
    }
}

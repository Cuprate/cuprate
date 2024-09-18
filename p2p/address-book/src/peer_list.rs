use std::collections::{BTreeMap, HashMap, HashSet};

use indexmap::IndexMap;
use rand::prelude::*;

use cuprate_p2p_core::{services::ZoneSpecificPeerListEntryBase, NetZoneAddress, NetworkZone};
use cuprate_pruning::{PruningSeed, CRYPTONOTE_MAX_BLOCK_HEIGHT};

#[cfg(test)]
pub(crate) mod tests;

/// A Peer list in the address book.
///
/// This could either be the white list or gray list.
#[derive(Debug)]
pub(crate) struct PeerList<Z: NetworkZone> {
    /// The peers with their peer data.
    pub peers: IndexMap<Z::Addr, ZoneSpecificPeerListEntryBase<Z::Addr>>,
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

impl<Z: NetworkZone> PeerList<Z> {
    /// Creates a new peer list.
    pub(crate) fn new(list: Vec<ZoneSpecificPeerListEntryBase<Z::Addr>>) -> Self {
        let mut peers = IndexMap::with_capacity(list.len());
        let mut pruning_seeds = BTreeMap::new();
        let mut ban_ids = HashMap::with_capacity(list.len());

        for peer in list {
            pruning_seeds
                .entry(peer.pruning_seed)
                .or_insert_with(Vec::new)
                .push(peer.adr);

            ban_ids
                .entry(peer.adr.ban_id())
                .or_insert_with(Vec::new)
                .push(peer.adr);

            peers.insert(peer.adr, peer);
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
        if self.peers.insert(peer.adr, peer).is_none() {
            #[expect(clippy::unwrap_or_default, reason = "It's more clear with this")]
            self.pruning_seeds
                .entry(peer.pruning_seed)
                .or_insert_with(Vec::new)
                .push(peer.adr);

            #[expect(clippy::unwrap_or_default)]
            self.ban_ids
                .entry(peer.adr.ban_id())
                .or_insert_with(Vec::new)
                .push(peer.adr);
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
        block_needed: Option<usize>,
        must_keep_peers: &HashSet<Z::Addr>,
    ) -> Option<ZoneSpecificPeerListEntryBase<Z::Addr>> {
        // Take a random peer and see if it's in the list of must_keep_peers, if it is try again.
        // TODO: improve this

        for _ in 0..3 {
            if let Some(needed_height) = block_needed {
                let (_, addresses_with_block) = self.pruning_seeds.iter().find(|(seed, _)| {
                    // TODO: factor in peer blockchain height?
                    seed.get_next_unpruned_block(needed_height, CRYPTONOTE_MAX_BLOCK_HEIGHT)
                        .expect("Block needed is higher than max block allowed.")
                        == needed_height
                })?;
                let n = r.gen_range(0..addresses_with_block.len());
                let peer = addresses_with_block[n];
                if must_keep_peers.contains(&peer) {
                    continue;
                }

                return self.remove_peer(&peer);
            }
            let len = self.len();

            if len == 0 {
                return None;
            }

            let n = r.gen_range(0..len);

            let (&key, _) = self.peers.get_index(n).unwrap();
            if !must_keep_peers.contains(&key) {
                return self.remove_peer(&key);
            }
        }

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

    /// Returns true if the list contains this peer.
    pub(crate) fn contains_peer(&self, peer: &Z::Addr) -> bool {
        self.peers.contains_key(peer)
    }

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
        let peer_eb = self.peers.swap_remove(peer)?;
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

    /// Tries to reduce the peer list to `new_len`.
    ///
    /// This function could keep the list bigger than `new_len` if `must_keep_peers`s length
    /// is larger than `new_len`, in that case we will remove as much as we can.
    pub(crate) fn reduce_list(&mut self, must_keep_peers: &HashSet<Z::Addr>, new_len: usize) {
        if new_len >= self.len() {
            return;
        }

        let target_removed = self.len() - new_len;
        let mut removed_count = 0;
        let mut peers_to_remove: Vec<Z::Addr> = Vec::with_capacity(target_removed);

        for peer_adr in self.peers.keys() {
            if removed_count >= target_removed {
                break;
            }
            if !must_keep_peers.contains(peer_adr) {
                peers_to_remove.push(*peer_adr);
                removed_count += 1;
            }
        }

        for peer_adr in peers_to_remove {
            let _ = self.remove_peer(&peer_adr);
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

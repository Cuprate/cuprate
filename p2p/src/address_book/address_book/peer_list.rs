//! This module contains the individual address books peer lists.
//!
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use cuprate_common::CRYPTONOTE_PRUNING_LOG_STRIPES;
use monero_wire::{messages::PeerListEntryBase, NetworkAddress};
use rand::Rng;

#[cfg(test)]
mod tests;

/// A Peer list in the address book.
///
/// This could either be the white list or gray list.
pub struct PeerList {
    /// The peers with their peer data.
    peers: HashMap<NetworkAddress, PeerListEntryBase>,
    /// An index of Pruning seed to address, so
    /// can quickly grab peers with the pruning seed
    /// we want.
    pruning_idxs: HashMap<u32, Vec<NetworkAddress>>,
    /// An index of [`ban_identifier`](NetworkAddress::ban_identifier) to Address
    /// to allow us to quickly remove baned peers.
    ban_id_idxs: HashMap<Vec<u8>, Vec<NetworkAddress>>,
}

impl<'a> Into<Vec<&'a PeerListEntryBase>> for &'a PeerList {
    fn into(self) -> Vec<&'a PeerListEntryBase> {
        self.peers.iter().map(|(_, peb)| peb).collect()
    }
}

impl PeerList {
    /// Creates a new peer list.
    pub fn new(list: Vec<PeerListEntryBase>) -> PeerList {
        let mut peers = HashMap::with_capacity(list.len());
        let mut pruning_idxs = HashMap::with_capacity(2 << CRYPTONOTE_PRUNING_LOG_STRIPES);
        let mut ban_id_idxs = HashMap::with_capacity(list.len()); // worse case, every peer has a different NetworkAddress and ban id

        for peer in list {
            peers.insert(peer.adr, peer);

            pruning_idxs
                .entry(peer.pruning_seed)
                .or_insert_with(Vec::new)
                .push(peer.adr);

            ban_id_idxs
                .entry(peer.adr.ban_identifier())
                .or_insert_with(Vec::new)
                .push(peer.adr);
        }
        PeerList {
            peers,
            pruning_idxs,
            ban_id_idxs,
        }
    }

    /// Gets the length of the peer list
    pub fn len(&self) -> usize {
        self.peers.len()
    }
    
    /// Adds a new peer to the peer list
    pub fn add_new_peer(&mut self, peer: PeerListEntryBase) {
        if let None = self.peers.insert(peer.adr, peer) {
            self.pruning_idxs
                .entry(peer.pruning_seed)
                .or_insert_with(Vec::new)
                .push(peer.adr);

            self.ban_id_idxs
                .entry(peer.adr.ban_identifier())
                .or_insert_with(Vec::new)
                .push(peer.adr);
        }
    }

    /// Gets a reference to a peer
    pub fn get_peer(&self, peer: &NetworkAddress) -> Option<&PeerListEntryBase> {
        self.peers.get(peer)
    }

    /// Returns an iterator over every peer in this peer list
    pub fn iter_all_peers(&self) -> impl Iterator<Item = &PeerListEntryBase> {
        self.peers.values()
    }

    /// Returns a random peer.
    /// If the pruning seed is specified then we will get a random peer with
    /// that pruning seed otherwise we will just get a random peer in the whole
    /// list.
    pub fn get_random_peer<R: Rng>(
        &self,
        r: &mut R,
        pruning_seed: Option<u32>,
    ) -> Option<&PeerListEntryBase> {
        if let Some(seed) = pruning_seed {
            let mut peers = self.get_peers_with_pruning(&seed)?;
            let len = self.len_by_seed(&seed);
            if len == 0 {
                None
            } else {
                let n = r.gen_range(0..len);

                peers.nth(n)
            }
        } else {
            let mut peers = self.iter_all_peers();
            let len = self.len();
            if len == 0 {
                None
            } else {
                let n = r.gen_range(0..len);

                peers.nth(n)
            }
        }
    }

    /// Returns a mutable reference to a peer.
    pub fn get_peer_mut(&mut self, peer: &NetworkAddress) -> Option<&mut PeerListEntryBase> {
        self.peers.get_mut(peer)
    }

    /// Returns true if the list contains this peer.
    pub fn contains_peer(&self, peer: &NetworkAddress) -> bool {
        self.peers.contains_key(peer)
    }

    /// Returns an iterator of peer info of peers with a specific pruning seed.
    fn get_peers_with_pruning(
        &self,
        seed: &u32,
    ) -> Option<impl Iterator<Item = &PeerListEntryBase>> {
        let addrs = self.pruning_idxs.get(seed)?;

        Some(addrs.iter().map(move |addr| {
            self.peers
                .get(addr)
                .expect("Address must be in peer list if we have an idx for it")
        }))
    }

    /// Removes a peer from the pruning idx
    ///
    /// MUST NOT BE USED ALONE
    fn remove_peer_pruning_idx(&mut self, peer: &PeerListEntryBase) {
        remove_peer_idx(&mut self.pruning_idxs, &peer.pruning_seed, &peer.adr)
    }

    /// Removes a peer from the ban idx
    ///
    /// MUST NOT BE USED ALONE
    fn remove_peer_ban_idx(&mut self, peer: &PeerListEntryBase) {
        remove_peer_idx(&mut self.ban_id_idxs, &peer.adr.ban_identifier(), &peer.adr)
    }

    /// Removes a peer from all the indexes
    ///
    /// MUST NOT BE USED ALONE
    fn remove_peer_from_all_idxs(&mut self, peer: &PeerListEntryBase) {
        self.remove_peer_ban_idx(peer);
        self.remove_peer_pruning_idx(peer);
    }

    /// Removes a peer from the peer list
    pub fn remove_peer(&mut self, peer: &NetworkAddress) -> Option<PeerListEntryBase> {
        let peer_eb = self.peers.remove(peer)?;
        self.remove_peer_from_all_idxs(&peer_eb);
        Some(peer_eb)
    }

    /// Removes all peers with a specific ban id.
    pub fn remove_peers_with_ban_id(&mut self, ban_id: &Vec<u8>) {
        let Some(addresses) = self.ban_id_idxs.get(ban_id) else {
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
    /// is larger than new_len, in that case we will remove as much as we can.
    pub fn reduce_list(&mut self, must_keep_peers: &HashSet<NetworkAddress>, new_len: usize) {
        if new_len >= self.len() {
            return;
        }

        let target_removed = self.len() - new_len;
        let mut removed_count = 0;
        let mut peers_to_remove: Vec<NetworkAddress> = Vec::with_capacity(target_removed);

        for (peer_adr, _) in &self.peers {
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
fn remove_peer_idx<T: Hash + Eq + PartialEq>(
    idx_map: &mut HashMap<T, Vec<NetworkAddress>>,
    idx: &T,
    addr: &NetworkAddress,
) {
    if let Some(peer_list) = idx_map.get_mut(idx) {
        if let Some(idx) = peer_list.iter().position(|peer_adr| peer_adr == addr) {
            peer_list.swap_remove(idx);
        } else {
            unreachable!("This function will only be called when the peer exists.");
        }
    } else {
        unreachable!("Index must exist if a peer has that index");
    }
}

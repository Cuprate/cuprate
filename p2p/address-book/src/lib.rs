use std::{
    collections::{HashMap, HashSet},
    vec,
};

use monero_wire::{messages::PeerListEntryBase, NetworkAddress};

struct PeerListIterator<'a> {
    list: &'a HashMap<NetworkAddress, PeerListEntryBase>,
    addrs: &'a [NetworkAddress],
    next_idx: usize,
}

impl<'a> PeerListIterator<'a> {
    pub fn new(list: &'a HashMap<NetworkAddress, PeerListEntryBase>, addrs: &'a [NetworkAddress]) -> Self {
        PeerListIterator {
            list,
            addrs,
            next_idx: 0,
        }
    }
}

impl<'a> Iterator for PeerListIterator<'a> {
    type Item = &'a PeerListEntryBase;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_idx += 1;
        self.list.get(self.addrs.get(self.next_idx - 1)?)
    }
}

struct PeerList {
    peers: HashMap<NetworkAddress, PeerListEntryBase>,
    pruning_idxs: HashMap<u32, Vec<NetworkAddress>>,
}

impl PeerList {
    pub fn new(list: Vec<PeerListEntryBase>) -> PeerList {
        let mut peers = HashMap::with_capacity(list.len());
        let mut pruning_idxs = HashMap::with_capacity(8);

        for peer in list {
            peers.insert(peer.adr, peer);

            let Some(idxs) = pruning_idxs.get_mut(&peer.pruning_seed) else {
                let _ = pruning_idxs.insert(peer.pruning_seed, vec![peer.adr]);
                continue;
            };
            idxs.push(peer.adr);
        }
        PeerList { peers, pruning_idxs }
    }

    pub fn len(&self) -> usize {
        self.peers.len()
    }

    pub fn get_peers_by_pruning_seed(&self, seed: &u32) -> Option<impl Iterator<Item = &PeerListEntryBase>> {
        let addrs = self.pruning_idxs.get(seed)?;
        Some(PeerListIterator::new(&self.peers, addrs))
    }

    fn remove_peer_pruning_idx(&mut self, peer: &PeerListEntryBase) {
        let peer_list = self
            .pruning_idxs
            .get_mut(&peer.pruning_seed)
            .expect("Pruning seed must exist if a peer has that seed");
        let mut i = 0;
        for peer_adr in peer_list.iter() {
            if peer_adr == &peer.adr {
                peer_list.remove(i);
                return;
            }
        }
        // this should be unreachable!() but no need
    }

    pub fn remove_peer(&mut self, peer: &NetworkAddress) -> Option<PeerListEntryBase> {
        let peer_eb = self.peers.remove(peer)?;
        self.remove_peer_pruning_idx(&peer_eb);
        Some(peer_eb)
    }

    pub fn reduce_list(&mut self, must_keep_peers: HashSet<NetworkAddress>, new_len: usize) {
        if new_len > self.len() {
            return;
        }
        let mut amt_to_remove = self.len() - new_len;
        let mut remove_list = Vec::with_capacity(amt_to_remove);

        for (peer_adr, _) in self.peers.iter() {
            if amt_to_remove == 0 || must_keep_peers.contains(peer_adr) {
                break;
            } else {
                remove_list.push(*peer_adr);
                amt_to_remove -= 1;
            }
        }

        for peer in remove_list {
            let _ = self.remove_peer(&peer);
        }
    }
}

pub struct AddressBook {
    white_list: PeerList,
    gray_list: PeerList,
}

impl AddressBook {
    pub fn new() {
        todo!()
    }

    fn len_white_list(&self) -> usize {
        self.white_list.len()
    }

    fn len_gray_list(&self) -> usize {
        self.gray_list.len()
    }
}

#[cfg(test)]
mod tests {
    use std::{vec, collections::HashSet};

    use monero_wire::{messages::PeerListEntryBase, NetworkAddress};

    use crate::PeerList;

    fn make_fake_peer_list(numb_o_peers: usize) -> PeerList {
        let mut peer_list = vec![PeerListEntryBase::default(); numb_o_peers];
        for (idx, peer) in peer_list.iter_mut().enumerate() {
            let NetworkAddress::IPv4(ip) =  &mut peer.adr else {panic!("this test requires default to be ipv4")};
            ip.m_ip += idx as u32;
        }

        PeerList::new(peer_list)
    }

    #[test]
    fn test_reduce_peer_list_length() {
        let mut peer_list = make_fake_peer_list(2090);
        let must_keep_peers = HashSet::new();

        let target_len = 2000;

        peer_list.reduce_list(must_keep_peers, target_len);

        assert_eq!(peer_list.len(), target_len);
    }

    #[test]
    fn test_reduce_peer_list_length_with_peers_we_need() {
        let mut peer_list = make_fake_peer_list(5000);
        let must_keep_peers = HashSet::from_iter(peer_list.peers.iter().map(|(adr, _)| *adr));

        let target_len = 4990;

        peer_list.reduce_list(must_keep_peers, target_len);

        // we can't remove any of the peers
        assert_eq!(peer_list.len(), 5000);
    }
}

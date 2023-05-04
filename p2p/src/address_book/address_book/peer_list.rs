use std::collections::{HashMap, HashSet};

use monero_wire::{messages::PeerListEntryBase, NetworkAddress};
use rand::Rng;

pub struct PeerList {
    peers: HashMap<NetworkAddress, PeerListEntryBase>,
    pruning_idxs: HashMap<u32, Vec<NetworkAddress>>,
}

impl<'a> Into<Vec<&'a PeerListEntryBase>> for &'a PeerList {
    fn into(self) -> Vec<&'a PeerListEntryBase> {
        self.peers.iter().map(|(_, peb)| peb).collect()
    }
}

impl PeerList {
    pub fn new(list: Vec<PeerListEntryBase>) -> PeerList {
        let mut peers = HashMap::with_capacity(list.len());
        let mut pruning_idxs = HashMap::with_capacity(8);

        for peer in list {
            peers.insert(peer.adr, peer);

            pruning_idxs
                .entry(peer.pruning_seed)
                .or_insert_with(Vec::new)
                .push(peer.adr);
        }
        PeerList {
            peers,
            pruning_idxs,
        }
    }

    pub fn len(&self) -> usize {
        self.peers.len()
    }

    pub fn add_new_peer(&mut self, peer: PeerListEntryBase) {
        if self.peers.insert(peer.adr, peer.clone()).is_none() {
            self.pruning_idxs
                .entry(peer.pruning_seed)
                .or_insert_with(Vec::new)
                .push(peer.adr);
        }
    }

    pub fn get_peer(&self, peer: &NetworkAddress) -> Option<&PeerListEntryBase> {
        self.peers.get(peer)
    }

    pub fn get_peer_by_idx(&self, n: usize) -> Option<&PeerListEntryBase> {
        self.peers.iter().nth(n).map(|(_, ret)| ret)
    }

    pub fn get_random_peer<R: Rng>(&self, r: &mut R) -> Option<&PeerListEntryBase> {
        let len = self.len();
        if len == 0 {
            None
        } else {
            let n = r.gen_range(0..len);

            self.get_peer_by_idx(n)
        }
    }

    pub fn get_peer_mut(&mut self, peer: &NetworkAddress) -> Option<&mut PeerListEntryBase> {
        self.peers.get_mut(peer)
    }

    pub fn contains_peer(&self, peer: &NetworkAddress) -> bool {
        self.peers.contains_key(peer)
    }

    pub fn get_peers_by_pruning_seed(
        &self,
        seed: &u32,
    ) -> Option<impl Iterator<Item = &PeerListEntryBase>> {
        let addrs = self.pruning_idxs.get(seed)?;
        Some(addrs.iter().filter_map(move |addr| self.peers.get(addr)))
    }

    fn remove_peer_pruning_idx(&mut self, peer: &PeerListEntryBase) {
        if let Some(peer_list) = self.pruning_idxs.get_mut(&peer.pruning_seed) {
            if let Some(idx) = peer_list.iter().position(|peer_adr| peer_adr == &peer.adr) {
                peer_list.remove(idx);
            } else {
                unreachable!("This function will only be called when the peer exists.");
            }
        } else {
            unreachable!("Pruning seed must exist if a peer has that seed.");
        }
    }

    pub fn remove_peer(&mut self, peer: &NetworkAddress) -> Option<PeerListEntryBase> {
        let peer_eb = self.peers.remove(peer)?;
        self.remove_peer_pruning_idx(&peer_eb);
        Some(peer_eb)
    }

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

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, vec};

    use monero_wire::{messages::PeerListEntryBase, NetworkAddress};
    use rand::Rng;

    use super::PeerList;

    fn make_fake_peer_list(numb_o_peers: usize) -> PeerList {
        let mut peer_list = vec![PeerListEntryBase::default(); numb_o_peers];
        for (idx, peer) in peer_list.iter_mut().enumerate() {
            let NetworkAddress::IPv4(ip) =  &mut peer.adr else {panic!("this test requires default to be ipv4")};
            ip.m_ip += idx as u32;
        }

        PeerList::new(peer_list)
    }

    fn make_fake_peer_list_with_random_pruning_seeds(numb_o_peers: usize) -> PeerList {
        let mut r = rand::thread_rng();

        let mut peer_list = vec![PeerListEntryBase::default(); numb_o_peers];
        for (idx, peer) in peer_list.iter_mut().enumerate() {
            let NetworkAddress::IPv4(ip) =  &mut peer.adr else {panic!("this test requires default to be ipv4")};
            ip.m_ip += idx as u32;

            peer.pruning_seed = if r.gen_bool(0.4) {
                0
            } else {
                r.gen_range(384..=391)
            };
        }

        PeerList::new(peer_list)
    }

    #[test]
    fn peer_list_reduce_length() {
        let mut peer_list = make_fake_peer_list(2090);
        let must_keep_peers = HashSet::new();

        let target_len = 2000;

        peer_list.reduce_list(&must_keep_peers, target_len);

        assert_eq!(peer_list.len(), target_len);
    }

    #[test]
    fn peer_list_reduce_length_with_peers_we_need() {
        let mut peer_list = make_fake_peer_list(500);
        let must_keep_peers = HashSet::from_iter(peer_list.peers.iter().map(|(adr, _)| *adr));

        let target_len = 49;

        peer_list.reduce_list(&must_keep_peers, target_len);

        // we can't remove any of the peers we said we need them all
        assert_eq!(peer_list.len(), 500);
    }

    #[test]
    fn peer_list_get_peers_by_pruning_seed() {
        let mut r = rand::thread_rng();

        let peer_list = make_fake_peer_list_with_random_pruning_seeds(1000);
        let seed = if r.gen_bool(0.4) {
            0
        } else {
            r.gen_range(384..=391)
        };

        let peers_with_seed = peer_list
            .get_peers_by_pruning_seed(&seed)
            .expect("If you hit this buy a lottery ticket");

        for peer in peers_with_seed {
            assert_eq!(peer.pruning_seed, seed);
        }

        assert_eq!(peer_list.len(), 1000);
    }

    #[test]
    fn peer_list_remove_specific_peer() {
        let mut peer_list = make_fake_peer_list_with_random_pruning_seeds(100);

        // generate peer at a random point in the list
        let mut peer = NetworkAddress::default();
        let NetworkAddress::IPv4(ip) =  &mut peer else {panic!("this test requires default to be ipv4")};
        ip.m_ip += 50;

        assert!(peer_list.remove_peer(&peer).is_some());

        let pruning_idxs = peer_list.pruning_idxs;
        let peers = peer_list.peers;

        for (_, addrs) in pruning_idxs {
            addrs.iter().for_each(|adr| assert!(adr != &peer))
        }

        assert!(!peers.contains_key(&peer));
    }

    #[test]
    fn peer_list_pruning_idxs_are_correct() {
        let peer_list = make_fake_peer_list_with_random_pruning_seeds(100);
        let mut total_len = 0;

        for (seed, list) in peer_list.pruning_idxs {
            for peer in list.iter() {
                assert_eq!(peer_list.peers.get(peer).unwrap().pruning_seed, seed);
                total_len += 1;
            }
        }

        assert_eq!(total_len, peer_list.peers.len())
    }

    #[test]
    fn peer_list_add_new_peer() {
        let mut peer_list = make_fake_peer_list(10);
        let mut new_peer = PeerListEntryBase::default();
        let NetworkAddress::IPv4(ip) =  &mut new_peer.adr else {panic!("this test requires default to be ipv4")};
        ip.m_ip += 50;

        peer_list.add_new_peer(new_peer.clone());

        assert_eq!(peer_list.len(), 11);
        assert_eq!(peer_list.get_peer(&new_peer.adr), Some(&new_peer));
        assert!(peer_list
            .pruning_idxs
            .get(&new_peer.pruning_seed)
            .unwrap()
            .contains(&new_peer.adr));
    }

    #[test]
    fn peer_list_add_existing_peer() {
        let mut peer_list = make_fake_peer_list(10);
        let existing_peer = peer_list
            .get_peer(&NetworkAddress::default())
            .unwrap()
            .clone();

        peer_list.add_new_peer(existing_peer.clone());

        assert_eq!(peer_list.len(), 10);
        assert_eq!(peer_list.get_peer(&existing_peer.adr), Some(&existing_peer));
    }

    #[test]
    fn peer_list_get_non_existent_peer() {
        let peer_list = make_fake_peer_list(10);
        let mut non_existent_peer = NetworkAddress::default();
        let NetworkAddress::IPv4(ip) =  &mut non_existent_peer else {panic!("this test requires default to be ipv4")};
        ip.m_ip += 50;

        assert_eq!(peer_list.get_peer(&non_existent_peer), None);
    }
}

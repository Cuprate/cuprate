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
        ip.m_port += r.gen_range(0..15);

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
        .get_peers_with_pruning(&seed)
        .expect("If you hit this buy a lottery ticket");

    for peer in peers_with_seed {
        assert_eq!(peer.pruning_seed, seed);
    }

    assert_eq!(peer_list.len(), 1000);
}

#[test]
fn peer_list_remove_specific_peer() {
    let mut peer_list = make_fake_peer_list_with_random_pruning_seeds(100);

    let peer = peer_list
        .get_random_peer(&mut rand::thread_rng(), None)
        .unwrap()
        .clone();

    assert!(peer_list.remove_peer(&peer.adr).is_some());

    let pruning_idxs = peer_list.pruning_idxs;
    let peers = peer_list.peers;

    for (_, addrs) in pruning_idxs {
        addrs.iter().for_each(|adr| assert_ne!(adr, &peer.adr))
    }

    assert!(!peers.contains_key(&peer.adr));
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

#[test]
fn peer_list_ban_peers() {
    let mut peer_list = make_fake_peer_list_with_random_pruning_seeds(100);
    let peer = peer_list
        .get_random_peer(&mut rand::thread_rng(), None)
        .unwrap();
    let ban_id = peer.adr.ban_identifier();
    assert!(peer_list.contains_peer(&peer.adr));
    assert_ne!(peer_list.ban_id_idxs.get(&ban_id).unwrap().len(), 0);
    peer_list.remove_peers_with_ban_id(&ban_id);
    assert_eq!(peer_list.ban_id_idxs.get(&ban_id).unwrap().len(), 0);
    for (addr, _) in peer_list.peers {
        assert_ne!(addr.ban_identifier(), ban_id);
    }
}

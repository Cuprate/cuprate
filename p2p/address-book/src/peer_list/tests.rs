use std::collections::HashSet;

use rand::Rng;

use monero_p2p::services::ZoneSpecificPeerListEntryBase;
use monero_pruning::PruningSeed;

use cuprate_test_utils::test_netzone::{TestNetZone, TestNetZoneAddr};
use monero_p2p::NetZoneAddress;

use super::PeerList;

fn make_fake_peer(
    id: u32,
    pruning_seed: Option<u32>,
) -> ZoneSpecificPeerListEntryBase<TestNetZoneAddr> {
    ZoneSpecificPeerListEntryBase {
        adr: TestNetZoneAddr(id),
        id: id as u64,
        last_seen: 0,
        pruning_seed: PruningSeed::decompress(pruning_seed.unwrap_or(0)).unwrap(),
        rpc_port: 0,
        rpc_credits_per_hash: 0,
    }
}

pub fn make_fake_peer_list(
    start_idx: u32,
    numb_o_peers: u32,
) -> PeerList<TestNetZone<true, true, true>> {
    let mut peer_list = Vec::with_capacity(numb_o_peers as usize);

    for idx in start_idx..(start_idx + numb_o_peers) {
        peer_list.push(make_fake_peer(idx, None))
    }

    PeerList::new(peer_list)
}

fn make_fake_peer_list_with_random_pruning_seeds(
    numb_o_peers: u32,
) -> PeerList<TestNetZone<true, true, true>> {
    let mut r = rand::thread_rng();

    let mut peer_list = Vec::with_capacity(numb_o_peers as usize);

    for idx in 0..numb_o_peers {
        peer_list.push(make_fake_peer(
            idx,
            Some(if r.gen_bool(0.4) {
                0
            } else {
                r.gen_range(384..=391)
            }),
        ))
    }
    PeerList::new(peer_list)
}

#[test]
fn peer_list_reduce_length() {
    let mut peer_list = make_fake_peer_list(0, 2090);
    let must_keep_peers = HashSet::new();

    let target_len = 2000;

    peer_list.reduce_list(&must_keep_peers, target_len);

    assert_eq!(peer_list.len(), target_len);
}

#[test]
fn peer_list_reduce_length_with_peers_we_need() {
    let mut peer_list = make_fake_peer_list(0, 500);
    let must_keep_peers = HashSet::from_iter(peer_list.peers.keys().copied());

    let target_len = 49;

    peer_list.reduce_list(&must_keep_peers, target_len);

    // we can't remove any of the peers we said we need them all
    assert_eq!(peer_list.len(), 500);
}

#[test]
fn peer_list_remove_specific_peer() {
    let mut peer_list = make_fake_peer_list_with_random_pruning_seeds(100);

    let peer = *peer_list
        .get_random_peer(&mut rand::thread_rng(), None)
        .unwrap();

    assert!(peer_list.remove_peer(&peer.adr).is_some());

    let pruning_idxs = peer_list.pruning_seeds;
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

    for (seed, list) in peer_list.pruning_seeds {
        for peer in list.iter() {
            assert_eq!(peer_list.peers.get(peer).unwrap().pruning_seed, seed);
            total_len += 1;
        }
    }

    assert_eq!(total_len, peer_list.peers.len())
}

#[test]
fn peer_list_add_new_peer() {
    let mut peer_list = make_fake_peer_list(0, 10);
    let new_peer = make_fake_peer(50, None);

    peer_list.add_new_peer(new_peer);

    assert_eq!(peer_list.len(), 11);
    assert_eq!(peer_list.get_peer(&new_peer.adr), Some(&new_peer));
    assert!(peer_list
        .pruning_seeds
        .get(&new_peer.pruning_seed)
        .unwrap()
        .contains(&new_peer.adr));
}

#[test]
fn peer_list_add_existing_peer() {
    let mut peer_list = make_fake_peer_list(0, 10);
    let existing_peer = *peer_list.get_peer(&TestNetZoneAddr(0)).unwrap();

    peer_list.add_new_peer(existing_peer);

    assert_eq!(peer_list.len(), 10);
    assert_eq!(peer_list.get_peer(&existing_peer.adr), Some(&existing_peer));
}

#[test]
fn peer_list_get_non_existent_peer() {
    let peer_list = make_fake_peer_list(0, 10);
    let non_existent_peer = TestNetZoneAddr(50);
    assert_eq!(peer_list.get_peer(&non_existent_peer), None);
}

#[test]
fn peer_list_get_peer_with_block() {
    let mut r = rand::thread_rng();

    let mut peer_list = make_fake_peer_list_with_random_pruning_seeds(100);
    peer_list.add_new_peer(make_fake_peer(101, Some(384)));

    let peer = peer_list
        .get_random_peer(&mut r, Some(1))
        .expect("We just added a peer with the correct seed");

    assert!(peer
        .pruning_seed
        .get_next_unpruned_block(1, 1_000_000)
        .is_ok())
}

#[test]
fn peer_list_ban_peers() {
    let mut peer_list = make_fake_peer_list_with_random_pruning_seeds(100);
    let peer = peer_list
        .get_random_peer(&mut rand::thread_rng(), None)
        .unwrap();
    let ban_id = peer.adr.ban_id();

    assert!(peer_list.contains_peer(&peer.adr));
    assert_ne!(peer_list.ban_ids.get(&ban_id).unwrap().len(), 0);
    peer_list.remove_peers_with_ban_id(&ban_id);
    assert_eq!(peer_list.ban_ids.get(&ban_id), None);
    for (addr, _) in peer_list.peers {
        assert_ne!(addr.ban_id(), ban_id);
    }
}

use std::{net::SocketAddr, path::PathBuf, time::Duration};

use futures::StreamExt;
use tokio::time::interval;

use cuprate_p2p_core::{handles::HandleBuilder, ClearNet, NetworkZone};
use cuprate_pruning::PruningSeed;

use super::{AddressBook, ConnectionPeerEntry, InternalPeerID};
use crate::{
    peer_list::tests::make_fake_peer_list, peer_list::PeerList, AddressBookConfig,
    AddressBookError, BorshNetworkZone,
};

use cuprate_test_utils::test_netzone::{TestNetZone, TestNetZoneAddr};

fn test_cfg<Z: NetworkZone>() -> AddressBookConfig<Z> {
    AddressBookConfig {
        max_white_list_length: 100,
        max_gray_list_length: 500,
        peer_store_directory: PathBuf::new(),
        peer_save_period: Duration::from_secs(60),
        our_own_address: None,
    }
}

fn empty_address_book<Z: BorshNetworkZone>() -> AddressBook<Z> {
    AddressBook {
        white_list: PeerList::new(vec![]),
        gray_list: PeerList::new(vec![]),
        anchor_list: Default::default(),
        connected_peers: Default::default(),
        banned_peers: Default::default(),
        banned_peers_queue: Default::default(),
        peer_save_task_handle: None,
        peer_save_interval: interval(Duration::from_secs(60)),
        cfg: test_cfg(),
    }
}

fn make_fake_address_book(numb_white: u32, numb_gray: u32) -> AddressBook<TestNetZone<true>> {
    AddressBook {
        white_list: make_fake_peer_list(0, numb_white),
        gray_list: make_fake_peer_list(numb_white, numb_gray),
        ..empty_address_book()
    }
}

#[tokio::test]
async fn take_random_peers() {
    let mut address_book = make_fake_address_book(50, 250);
    let peer = address_book.take_random_white_peer(None).unwrap();
    assert!(!address_book.white_list.contains_peer(&peer.adr));
    assert!(!address_book.gray_list.contains_peer(&peer.adr));

    let peer = address_book.take_random_gray_peer(None).unwrap();
    assert!(!address_book.white_list.contains_peer(&peer.adr));
    assert!(!address_book.gray_list.contains_peer(&peer.adr));
}

#[tokio::test]
async fn get_white_peers() {
    let address_book = make_fake_address_book(100, 0);
    let peers = address_book.get_white_peers(50);
    assert_eq!(peers.len(), 50);
    let peers = address_book.get_white_peers(60);
    assert_eq!(peers.len(), 60);
    for window in peers.windows(2) {
        assert_ne!(window[0], window[1]);
    }

    let address_book = make_fake_address_book(45, 0);
    let peers = address_book.get_white_peers(50);
    assert_eq!(peers.len(), 45);
    let peers = address_book.get_white_peers(60);
    assert_eq!(peers.len(), 45);
    for window in peers.windows(2) {
        assert_ne!(window[0], window[1]);
    }
}

#[tokio::test]
async fn add_new_peer_already_connected() {
    let mut address_book = make_fake_address_book(0, 0);

    let (_, handle) = HandleBuilder::default().build();

    address_book
        .handle_new_connection(
            InternalPeerID::KnownAddr(TestNetZoneAddr(1)),
            ConnectionPeerEntry {
                addr: None,
                id: 0,
                handle,
                pruning_seed: PruningSeed::decompress(385).unwrap(),
                rpc_port: 0,
                rpc_credits_per_hash: 0,
            },
        )
        .unwrap();

    let (_, handle) = HandleBuilder::default().build();

    assert_eq!(
        address_book.handle_new_connection(
            InternalPeerID::KnownAddr(TestNetZoneAddr(1)),
            ConnectionPeerEntry {
                addr: None,
                id: 0,
                handle,
                pruning_seed: PruningSeed::decompress(385).unwrap(),
                rpc_port: 0,
                rpc_credits_per_hash: 0,
            },
        ),
        Err(AddressBookError::PeerAlreadyConnected)
    );
}

#[tokio::test]
async fn banned_peer_removed_from_peer_lists() {
    let mut address_book = make_fake_address_book(100, 0);

    assert_eq!(address_book.banned_peers.len(), 0);
    assert_eq!(address_book.white_list.len(), 100);

    address_book.ban_peer(TestNetZoneAddr(1), Duration::from_secs(1));
    assert_eq!(address_book.banned_peers.len(), 1);
    assert_eq!(address_book.white_list.len(), 99);

    address_book.ban_peer(TestNetZoneAddr(1), Duration::from_secs(1));
    assert_eq!(address_book.banned_peers.len(), 1);
    assert_eq!(address_book.white_list.len(), 99);

    address_book.ban_peer(TestNetZoneAddr(1), Duration::from_secs(1));
    assert_eq!(address_book.banned_peers.len(), 1);
    assert_eq!(address_book.white_list.len(), 99);

    address_book.ban_peer(TestNetZoneAddr(5), Duration::from_secs(100));
    assert_eq!(address_book.banned_peers.len(), 2);
    assert_eq!(address_book.white_list.len(), 98);

    assert_eq!(
        address_book
            .banned_peers_queue
            .next()
            .await
            .unwrap()
            .into_inner(),
        TestNetZoneAddr(1)
    );
}

#[tokio::test]
async fn ban_peer_after_failed_connection() {
    let mut book = empty_address_book::<ClearNet>();

    let dialled: SocketAddr = "1.2.3.4:18080".parse().unwrap();
    let other: SocketAddr = "1.2.3.4:45678".parse().unwrap();

    let entry = |handle, pruning_seed| ConnectionPeerEntry {
        addr: Some(dialled),
        id: 0,
        handle,
        pruning_seed,
        rpc_port: 0,
        rpc_credits_per_hash: 0,
    };

    let (_guard, handle) = HandleBuilder::default().build();
    book.handle_new_connection(
        InternalPeerID::KnownAddr(dialled),
        entry(handle.clone(), PruningSeed::NotPruned),
    )
    .unwrap();

    handle.send_close_signal();
    book.poll_connected_peers();

    let (_guard, handle) = HandleBuilder::default().build();
    assert_eq!(
        book.handle_new_connection(
            InternalPeerID::KnownAddr(dialled),
            entry(handle, PruningSeed::decompress(385).unwrap()),
        ),
        Err(AddressBookError::PeersDataChanged("Pruning seed"))
    );

    let (_guard, handle) = HandleBuilder::default().build();
    book.handle_new_connection(
        InternalPeerID::KnownAddr(other),
        ConnectionPeerEntry {
            addr: None,
            id: 0,
            handle,
            pruning_seed: PruningSeed::NotPruned,
            rpc_port: 0,
            rpc_credits_per_hash: 0,
        },
    )
    .unwrap();

    book.ban_peer(other, Duration::from_secs(1));
    assert_eq!(book.banned_peers.len(), 1);
}

#[tokio::test]
async fn ban_peer_closes_all_with_ban_id() {
    let mut book = empty_address_book::<ClearNet>();

    let first: SocketAddr = "1.2.3.4:18080".parse().unwrap();
    let second: SocketAddr = "1.2.3.4:45678".parse().unwrap();
    let unrelated: SocketAddr = "5.6.7.8:18080".parse().unwrap();

    let mut handles = Vec::new();
    for addr in [first, second, unrelated] {
        let (guard, handle) = HandleBuilder::default().build();
        book.handle_new_connection(
            InternalPeerID::KnownAddr(addr),
            ConnectionPeerEntry {
                addr: Some(addr),
                id: 0,
                handle: handle.clone(),
                pruning_seed: PruningSeed::NotPruned,
                rpc_port: 0,
                rpc_credits_per_hash: 0,
            },
        )
        .unwrap();
        handles.push((addr, guard, handle));
    }

    assert_eq!(book.anchor_list.len(), 3);

    book.ban_peer(first, Duration::from_secs(1));

    for (addr, _guard, handle) in &handles {
        if addr.ip() == first.ip() {
            assert!(handle.is_closed(), "{addr} shares the ban ID, should close");
            assert!(!book.anchor_list.contains(addr));
        } else {
            assert!(!handle.is_closed(), "{addr} should be untouched");
            assert!(book.anchor_list.contains(addr));
        }
    }
}

#[tokio::test]
async fn anchor_removed_for_inbound_peer() {
    let mut book = empty_address_book::<ClearNet>();

    let source: SocketAddr = "1.2.3.4:45678".parse().unwrap();
    let reachable: SocketAddr = "1.2.3.4:18080".parse().unwrap();

    let connect = |book: &mut AddressBook<ClearNet>, handle| {
        book.handle_new_connection(
            InternalPeerID::KnownAddr(source),
            ConnectionPeerEntry {
                addr: Some(reachable),
                id: 0,
                handle,
                pruning_seed: PruningSeed::NotPruned,
                rpc_port: 0,
                rpc_credits_per_hash: 0,
            },
        )
        .unwrap();
    };

    let (_guard, handle) = HandleBuilder::default().build();
    connect(&mut book, handle);
    assert!(book.anchor_list.contains(&reachable));

    book.ban_peer(source, Duration::from_secs(1));
    assert!(book.anchor_list.is_empty());

    book.poll_connected_peers();
    book.banned_peers.clear();
    assert!(book.connected_peers.is_empty());

    let (_guard, handle) = HandleBuilder::default().build();
    connect(&mut book, handle.clone());
    assert!(book.anchor_list.contains(&reachable));

    handle.send_close_signal();
    book.poll_connected_peers();
    assert!(book.anchor_list.is_empty());
    assert!(book.connected_peers.is_empty());
}

#[tokio::test]
async fn anchor_kept_while_another_connection_remains() {
    let mut book = empty_address_book::<ClearNet>();

    let reachable: SocketAddr = "1.2.3.4:18080".parse().unwrap();
    let inbound_source: SocketAddr = "1.2.3.4:45678".parse().unwrap();

    let mut handles = Vec::new();
    for internal_addr in [reachable, inbound_source] {
        let (guard, handle) = HandleBuilder::default().build();
        book.handle_new_connection(
            InternalPeerID::KnownAddr(internal_addr),
            ConnectionPeerEntry {
                addr: Some(reachable),
                id: 0,
                handle: handle.clone(),
                pruning_seed: PruningSeed::NotPruned,
                rpc_port: 0,
                rpc_credits_per_hash: 0,
            },
        )
        .unwrap();
        handles.push((guard, handle));
    }
    assert!(book.anchor_list.contains(&reachable));

    handles[0].1.send_close_signal();
    book.poll_connected_peers();
    assert_eq!(book.connected_peers.len(), 1);
    assert!(book.anchor_list.contains(&reachable));

    handles[1].1.send_close_signal();
    book.poll_connected_peers();
    assert!(book.connected_peers.is_empty());
    assert!(book.anchor_list.is_empty());
}

use std::{path::PathBuf, sync::Arc, time::Duration};

use futures::StreamExt;
use tokio::sync::Semaphore;
use tokio::time::interval;

use monero_p2p::handles::HandleBuilder;
use monero_pruning::PruningSeed;

use super::{AddressBook, ConnectionPeerEntry, InternalPeerID};
use crate::{peer_list::tests::make_fake_peer_list, AddressBookError, Config};

use cuprate_test_utils::test_netzone::{TestNetZone, TestNetZoneAddr};

fn test_cfg() -> Config {
    Config {
        max_white_list_length: 100,
        max_gray_list_length: 500,
        peer_store_file: PathBuf::new(),
        peer_save_period: Duration::from_secs(60),
    }
}

fn make_fake_address_book(
    numb_white: u32,
    numb_gray: u32,
) -> AddressBook<TestNetZone<true, true, true>> {
    let white_list = make_fake_peer_list(0, numb_white);
    let gray_list = make_fake_peer_list(numb_white, numb_gray);

    AddressBook {
        white_list,
        gray_list,
        anchor_list: Default::default(),
        connected_peers: Default::default(),
        connected_peers_ban_id: Default::default(),
        banned_peers: Default::default(),
        banned_peers_fut: Default::default(),
        peer_save_task_handle: None,
        peer_save_interval: interval(Duration::from_secs(60)),
        cfg: test_cfg(),
    }
}

#[tokio::test]
async fn get_random_peers() {
    let address_book = make_fake_address_book(50, 250);
    let peer = address_book.get_random_white_peer(None).unwrap();
    assert!(address_book.white_list.contains_peer(&peer.adr));
    assert!(!address_book.gray_list.contains_peer(&peer.adr));

    let peer = address_book.get_random_gray_peer(None).unwrap();
    assert!(!address_book.white_list.contains_peer(&peer.adr));
    assert!(address_book.gray_list.contains_peer(&peer.adr));
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

    let semaphore = Arc::new(Semaphore::new(10));

    let (_, handle, _) = HandleBuilder::default()
        .with_permit(semaphore.clone().try_acquire_owned().unwrap())
        .build();

    address_book
        .handle_new_connection(
            InternalPeerID::KnownAddr(TestNetZoneAddr(1)),
            ConnectionPeerEntry {
                addr: None,
                id: 0,
                handle,
                pruning_seed: PruningSeed::try_from(385).unwrap(),
                rpc_port: 0,
                rpc_credits_per_hash: 0,
            },
        )
        .unwrap();

    let (_, handle, _) = HandleBuilder::default()
        .with_permit(semaphore.try_acquire_owned().unwrap())
        .build();

    assert_eq!(
        address_book.handle_new_connection(
            InternalPeerID::KnownAddr(TestNetZoneAddr(1)),
            ConnectionPeerEntry {
                addr: None,
                id: 0,
                handle,
                pruning_seed: PruningSeed::try_from(385).unwrap(),
                rpc_port: 0,
                rpc_credits_per_hash: 0,
            },
        ),
        Err(AddressBookError::PeerAlreadyConnected)
    )
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
        address_book.banned_peers_fut.next().await.unwrap(),
        TestNetZoneAddr(1)
    )
}

use std::{
    future::poll_fn,
    net::SocketAddr,
    sync::{Arc, Mutex},
    task::Poll,
};

use tokio::sync::mpsc;
use tower::{Service, ServiceExt};

use cuprate_p2p_core::{
    client::{mock_client, Client, InternalPeerID, PeerInformation},
    handles::{ConnectionHandle, HandleBuilder},
    ClearNet, ConnectionDirection, PeerRequest, PeerResponse, ProtocolResponse,
};
use cuprate_pruning::PruningSeed;
use cuprate_wire::{common::PeerSupportFlags, BasicNodeData, CoreSyncData};

use super::{ClientDropGuard, PeerSet, PeerSetRequest, PeerSetResponse};

fn mock_peer(
    id: InternalPeerID<SocketAddr>,
    direction: ConnectionDirection,
) -> (Client<ClearNet>, ConnectionHandle) {
    let (guard, handle) = HandleBuilder::new().build();

    let info = PeerInformation {
        id,
        basic_node_data: BasicNodeData {
            my_port: 0,
            network_id: [0; 16],
            peer_id: 0,
            support_flags: PeerSupportFlags::FLUFFY_BLOCKS,
            rpc_port: 0,
            rpc_credits_per_hash: 0,
        },
        handle: handle.clone(),
        direction,
        pruning_seed: PruningSeed::NotPruned,
        core_sync_data: Arc::new(Mutex::new(CoreSyncData {
            cumulative_difficulty: 1,
            cumulative_difficulty_top64: 0,
            current_height: 1,
            pruning_seed: 0,
            top_id: [0; 32],
            top_version: 0,
        })),
    };

    let svc = tower::service_fn(|_: PeerRequest| async {
        Ok::<_, tower::BoxError>(PeerResponse::Protocol(ProtocolResponse::NA))
    });

    (mock_client(info, guard, svc), handle)
}

async fn stem_peer(peer_set: &mut PeerSet<ClearNet>) -> Option<ClientDropGuard<ClearNet>> {
    let PeerSetResponse::StemPeer(stem) = peer_set
        .ready()
        .await
        .unwrap()
        .call(PeerSetRequest::StemPeer)
        .await
        .unwrap()
    else {
        panic!("peer set returned the wrong response");
    };

    stem
}

/// Closes an outbound connection and gives its id to a new connection before the peer set removes the old one.
async fn reuse_closed_peer_id(new_direction: ConnectionDirection) {
    let id = InternalPeerID::KnownAddr("10.0.0.1:18080".parse::<SocketAddr>().unwrap());

    let (tx, rx) = mpsc::channel(4);
    let mut peer_set = PeerSet::<ClearNet>::new(rx);

    let (old, old_handle) = mock_peer(id, ConnectionDirection::Outbound);
    tx.send(old).await.unwrap();
    peer_set.ready().await.unwrap();

    old_handle.send_close_signal();
    let (new, new_handle) = mock_peer(id, new_direction);
    tx.send(new).await.unwrap();

    // Take in the new connection before removing closed ones, whatever order `poll_ready` uses.
    poll_fn(|cx| {
        peer_set.poll_new_peers(cx);
        peer_set.remove_dead_peers(cx);
        Poll::Ready(())
    })
    .await;

    let peer = &peer_set.peers[&id];
    assert_eq!(peer.client.info.direction, new_direction);
    assert!(!peer.client.info.handle.is_closed());

    // Only outbound peers are used for stemming.
    assert_eq!(
        stem_peer(&mut peer_set).await.is_some(),
        new_direction == ConnectionDirection::Outbound
    );

    new_handle.send_close_signal();
    peer_set.ready().await.unwrap();
    assert!(peer_set.peers.is_empty());
}

#[tokio::test]
async fn peer_id_reused_by_same_direction() {
    reuse_closed_peer_id(ConnectionDirection::Outbound).await;
}

#[tokio::test]
async fn peer_id_reused_by_opposite_direction() {
    reuse_closed_peer_id(ConnectionDirection::Inbound).await;
}

/// A connection that closed while queued arrives after a new connection with the same id.
#[tokio::test]
async fn closed_connection_does_not_replace_reused_id() {
    let id = InternalPeerID::KnownAddr("10.0.0.2:18080".parse::<SocketAddr>().unwrap());

    let (tx, rx) = mpsc::channel(4);
    let mut peer_set = PeerSet::<ClearNet>::new(rx);

    let (old, old_handle) = mock_peer(id, ConnectionDirection::Outbound);
    old_handle.send_close_signal();
    let (new, _) = mock_peer(id, ConnectionDirection::Inbound);
    tx.send(new).await.unwrap();
    tx.send(old).await.unwrap();
    peer_set.ready().await.unwrap();

    let peer = &peer_set.peers[&id];
    assert_eq!(peer.client.info.direction, ConnectionDirection::Inbound);
    assert!(!peer.client.info.handle.is_closed());
    assert!(stem_peer(&mut peer_set).await.is_none());
}

#[tokio::test]
async fn stem_peer_is_an_unused_outbound_peer() {
    let (tx, rx) = mpsc::channel(4);
    let mut peer_set = PeerSet::<ClearNet>::new(rx);

    for (addr, direction) in [
        ("10.0.0.3:18080", ConnectionDirection::Inbound),
        ("10.0.0.4:18080", ConnectionDirection::Outbound),
    ] {
        let id = InternalPeerID::KnownAddr(addr.parse::<SocketAddr>().unwrap());
        tx.send(mock_peer(id, direction).0).await.unwrap();
    }

    let stem = stem_peer(&mut peer_set).await.expect("no stem peer");
    assert_eq!(stem.info.direction, ConnectionDirection::Outbound);

    // The only outbound peer is already being used for stemming.
    assert!(stem_peer(&mut peer_set).await.is_none());

    drop(stem);
    assert!(stem_peer(&mut peer_set).await.is_some());
}

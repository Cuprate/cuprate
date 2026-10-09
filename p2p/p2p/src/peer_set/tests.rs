use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use futures::FutureExt;
use tokio::sync::mpsc;
use tower::{Service, ServiceExt};

use cuprate_p2p_core::{
    client::{mock_client, Client, InternalPeerID, PeerInformation},
    handles::{ConnectionHandle, HandleBuilder},
    ClearNet, ConnectionDirection, PeerRequest, PeerResponse, ProtocolResponse,
};
use cuprate_pruning::PruningSeed;
use cuprate_wire::{common::PeerSupportFlags, BasicNodeData, CoreSyncData};

use super::{PeerSet, PeerSetRequest, PeerSetResponse};

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

    let svc = tower::service_fn(|_: PeerRequest| {
        async { Ok::<_, tower::BoxError>(PeerResponse::Protocol(ProtocolResponse::NA)) }.boxed()
    });

    (mock_client::<ClearNet, _>(info, guard, svc), handle)
}

#[tokio::test]
async fn peer_id_reused_by_opposite_direction() {
    let id = InternalPeerID::KnownAddr("10.0.0.1:18080".parse::<SocketAddr>().unwrap());

    let (tx, rx) = mpsc::channel(4);
    let mut peer_set = PeerSet::<ClearNet>::new(rx);

    let (outbound, outbound_handle) = mock_peer(id, ConnectionDirection::Outbound);
    tx.send(outbound).await.unwrap();
    peer_set.ready().await.unwrap();
    assert_eq!(peer_set.peers.len(), 1);

    // Close it without polling the peer set, so the close future stays pending.
    outbound_handle.send_close_signal();

    let (inbound, inbound_handle) = mock_peer(id, ConnectionDirection::Inbound);
    tx.send(inbound).await.unwrap();
    peer_set.ready().await.unwrap();

    // The stale close future must not evict the new connection.
    assert!(!inbound_handle.is_closed());
    assert_eq!(peer_set.peers.len(), 1);

    let PeerSetResponse::StemPeer(stem) = peer_set.call(PeerSetRequest::StemPeer).await.unwrap()
    else {
        panic!("peer set returned the wrong response");
    };
    assert!(stem.is_none());

    inbound_handle.send_close_signal();
    peer_set.ready().await.unwrap();
    assert!(peer_set.peers.is_empty());
}

#[tokio::test]
async fn outbound_peer_is_used_for_stem() {
    let id = InternalPeerID::KnownAddr("10.0.0.2:18080".parse::<SocketAddr>().unwrap());

    let (tx, rx) = mpsc::channel(4);
    let mut peer_set = PeerSet::<ClearNet>::new(rx);

    let (outbound, _handle) = mock_peer(id, ConnectionDirection::Outbound);
    tx.send(outbound).await.unwrap();
    peer_set.ready().await.unwrap();

    let PeerSetResponse::StemPeer(stem) = peer_set.call(PeerSetRequest::StemPeer).await.unwrap()
    else {
        panic!("peer set returned the wrong response");
    };
    assert!(stem.is_some());
}

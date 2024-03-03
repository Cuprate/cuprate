use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use tokio::{
    io::{duplex, split},
    sync::{broadcast, Semaphore},
    time::timeout,
};
use tokio_util::codec::{FramedRead, FramedWrite};
use tower::{Service, ServiceExt};

use cuprate_helper::network::Network;
use monero_wire::{common::PeerSupportFlags, BasicNodeData, MoneroWireCodec};

use monero_p2p::{
    client::{ConnectRequest, Connector, DoHandshakeRequest, HandShaker, InternalPeerID},
    network_zones::{ClearNet, ClearNetServerCfg},
    ConnectionDirection, NetworkZone,
};

use cuprate_test_utils::{
    monerod::monerod,
    test_netzone::{TestNetZone, TestNetZoneAddr},
};

mod utils;
use utils::*;

#[tokio::test]
async fn handshake_cuprate_to_cuprate() {
    // Tests a Cuprate <-> Cuprate handshake by making 2 handshake services and making them talk to
    // each other.

    let (broadcast_tx, _) = broadcast::channel(1); // this isn't actually used in this test.
    let semaphore = Arc::new(Semaphore::new(10));
    let permit_1 = semaphore.clone().acquire_owned().await.unwrap();
    let permit_2 = semaphore.acquire_owned().await.unwrap();

    let our_basic_node_data_1 = BasicNodeData {
        my_port: 0,
        network_id: Network::Mainnet.network_id().into(),
        peer_id: 87980,
        // TODO: This fails if the support flags are empty (0)
        support_flags: PeerSupportFlags::from(1_u32),
        rpc_port: 0,
        rpc_credits_per_hash: 0,
    };
    // make sure both node IDs are different
    let mut our_basic_node_data_2 = our_basic_node_data_1.clone();
    our_basic_node_data_2.peer_id = 2344;

    let mut handshaker_1 = HandShaker::<TestNetZone<true, true, true>, _, _, _>::new(
        DummyAddressBook,
        DummyCoreSyncSvc,
        DummyPeerRequestHandlerSvc,
        broadcast_tx.clone(),
        our_basic_node_data_1,
    );

    let mut handshaker_2 = HandShaker::<TestNetZone<true, true, true>, _, _, _>::new(
        DummyAddressBook,
        DummyCoreSyncSvc,
        DummyPeerRequestHandlerSvc,
        broadcast_tx.clone(),
        our_basic_node_data_2,
    );

    let (p1, p2) = duplex(50_000);

    let (p1_receiver, p1_sender) = split(p1);
    let (p2_receiver, p2_sender) = split(p2);

    let p1_handshake_req = DoHandshakeRequest {
        addr: InternalPeerID::KnownAddr(TestNetZoneAddr(888)),
        peer_stream: FramedRead::new(p2_receiver, MoneroWireCodec::default()),
        peer_sink: FramedWrite::new(p2_sender, MoneroWireCodec::default()),
        direction: ConnectionDirection::OutBound,
        permit: permit_1,
    };

    let p2_handshake_req = DoHandshakeRequest {
        addr: InternalPeerID::KnownAddr(TestNetZoneAddr(444)),
        peer_stream: FramedRead::new(p1_receiver, MoneroWireCodec::default()),
        peer_sink: FramedWrite::new(p1_sender, MoneroWireCodec::default()),
        direction: ConnectionDirection::InBound,
        permit: permit_2,
    };

    let p1 = tokio::spawn(async move {
        handshaker_1
            .ready()
            .await
            .unwrap()
            .call(p1_handshake_req)
            .await
            .unwrap()
    });

    let p2 = tokio::spawn(async move {
        handshaker_2
            .ready()
            .await
            .unwrap()
            .call(p2_handshake_req)
            .await
            .unwrap()
    });

    let (res1, res2) = futures::join!(p1, p2);
    res1.unwrap();
    res2.unwrap();
}

#[tokio::test]
async fn handshake_cuprate_to_monerod() {
    let (broadcast_tx, _) = broadcast::channel(1); // this isn't actually used in this test.
    let semaphore = Arc::new(Semaphore::new(10));
    let permit = semaphore.acquire_owned().await.unwrap();

    let monerod = monerod(["--fixed-difficulty=1", "--out-peers=0"]).await;

    let our_basic_node_data = BasicNodeData {
        my_port: 0,
        network_id: Network::Mainnet.network_id().into(),
        peer_id: 87980,
        support_flags: PeerSupportFlags::from(1_u32),
        rpc_port: 0,
        rpc_credits_per_hash: 0,
    };

    let handshaker = HandShaker::<ClearNet, _, _, _>::new(
        DummyAddressBook,
        DummyCoreSyncSvc,
        DummyPeerRequestHandlerSvc,
        broadcast_tx,
        our_basic_node_data,
    );

    let mut connector = Connector::new(handshaker);

    connector
        .ready()
        .await
        .unwrap()
        .call(ConnectRequest {
            addr: monerod.p2p_addr(),
            permit,
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn handshake_monerod_to_cuprate() {
    let (broadcast_tx, _) = broadcast::channel(1); // this isn't actually used in this test.
    let semaphore = Arc::new(Semaphore::new(10));
    let permit = semaphore.acquire_owned().await.unwrap();

    let our_basic_node_data = BasicNodeData {
        my_port: 18081,
        network_id: Network::Mainnet.network_id().into(),
        peer_id: 87980,
        support_flags: PeerSupportFlags::from(1_u32),
        rpc_port: 0,
        rpc_credits_per_hash: 0,
    };

    let mut handshaker = HandShaker::<ClearNet, _, _, _>::new(
        DummyAddressBook,
        DummyCoreSyncSvc,
        DummyPeerRequestHandlerSvc,
        broadcast_tx,
        our_basic_node_data,
    );

    let addr = "127.0.0.1:18081".parse().unwrap();

    let mut listener = ClearNet::incoming_connection_listener(ClearNetServerCfg { addr })
        .await
        .unwrap();

    let _monerod = monerod(["--add-exclusive-node=127.0.0.1:18081"]).await;

    // Put a timeout on this just in case monerod doesn't make the connection to us.
    let next_connection_fut = timeout(Duration::from_secs(30), listener.next());

    if let Some(Ok((addr, stream, sink))) = next_connection_fut.await.unwrap() {
        let _ = handshaker
            .ready()
            .await
            .unwrap()
            .call(DoHandshakeRequest {
                addr: InternalPeerID::KnownAddr(addr.unwrap()), // This is clear net all addresses are known.
                peer_stream: stream,
                peer_sink: sink,
                direction: ConnectionDirection::InBound,
                permit,
            })
            .await
            .unwrap();
    } else {
        panic!("Failed to receive connection from monerod.");
    };
}

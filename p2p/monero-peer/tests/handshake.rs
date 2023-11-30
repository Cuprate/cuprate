use std::{net::SocketAddr, str::FromStr};

use futures::{channel::mpsc, StreamExt};
use tower::{Service, ServiceExt};
use tracing::level_filters::LevelFilter;

use cuprate_common::Network;
use monero_wire::{common::PeerSupportFlags, BasicNodeData};

use monero_peer::{
    client::{ConnectRequest, Connector, DoHandshakeRequest, HandShaker},
    network_zones::ClearNet,
    ConnectionDirection,
};

use cuprate_test_utils::test_netzone::{TestNetZone, TestNetZoneAddr};

mod utils;
use utils::*;

#[tokio::test]
async fn handshake_cuprate_to_cuprate() {
    // Tests a Cuprate <-> Cuprate handshake by making 2 handshake services and making them talk to
    // each other.
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::TRACE)
        .init();

    let our_basic_node_data_1 = BasicNodeData {
        my_port: 0,
        network_id: Network::Mainnet.network_id(),
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
        our_basic_node_data_1,
    );

    let mut handshaker_2 = HandShaker::<TestNetZone<true, true, true>, _, _, _>::new(
        DummyAddressBook,
        DummyCoreSyncSvc,
        DummyPeerRequestHandlerSvc,
        our_basic_node_data_2,
    );

    let (p1_sender, p2_receiver) = mpsc::channel(5);
    let (p2_sender, p1_receiver) = mpsc::channel(5);

    let p1_handshake_req = DoHandshakeRequest {
        addr: TestNetZoneAddr(888),
        peer_stream: p2_receiver.map(Ok).boxed(),
        peer_sink: p2_sender.into(),
        direction: ConnectionDirection::OutBound,
    };

    let p2_handshake_req = DoHandshakeRequest {
        addr: TestNetZoneAddr(444),
        peer_stream: p1_receiver.boxed().map(Ok).boxed(),
        peer_sink: p1_sender.into(),
        direction: ConnectionDirection::InBound,
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
async fn handshake() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::TRACE)
        .init();

    let addr = "127.0.0.1:18080";

    let our_basic_node_data = BasicNodeData {
        my_port: 0,
        network_id: Network::Mainnet.network_id(),
        peer_id: 87980,
        support_flags: PeerSupportFlags::from(1_u32),
        rpc_port: 0,
        rpc_credits_per_hash: 0,
    };

    let handshaker = HandShaker::<ClearNet, _, _, _>::new(
        DummyAddressBook,
        DummyCoreSyncSvc,
        DummyPeerRequestHandlerSvc,
        our_basic_node_data,
    );

    let mut connector = Connector::new(handshaker);

    connector
        .ready()
        .await
        .unwrap()
        .call(ConnectRequest {
            addr: SocketAddr::from_str(addr).unwrap(),
        })
        .await
        .unwrap();
}

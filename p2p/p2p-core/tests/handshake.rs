#![expect(unused_crate_dependencies, reason = "external test module")]

use std::time::Duration;

use futures::StreamExt;
use tokio::{
    io::{duplex, split},
    time::timeout,
};
use tokio_util::codec::{FramedRead, FramedWrite};
use tower::{Service, ServiceExt};

use cuprate_helper::network::Network;
use cuprate_test_utils::{
    monerod::monerod,
    test_netzone::{TestNetZone, TestNetZoneAddr},
};
use cuprate_wire::{common::PeerSupportFlags, BasicNodeData, MoneroWireCodec};

use cuprate_p2p_core::{
    client::{
        handshaker::HandshakerBuilder, ConnectRequest, Connector, DoHandshakeRequest,
        InternalPeerID,
    },
    transports::{DummyTransport, Tcp, TcpServerConfig},
    ClearNet, ConnectionDirection, Transport,
};

#[tokio::test]
#[expect(clippy::significant_drop_tightening)]
async fn handshake_cuprate_to_cuprate() {
    // Tests a Cuprate <-> Cuprate handshake by making 2 handshake services and making them talk to
    // each other.
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

    let mut handshaker_1 =
        HandshakerBuilder::<TestNetZone<true>, DummyTransport>::new(our_basic_node_data_1, ())
            .build();

    let mut handshaker_2 =
        HandshakerBuilder::<TestNetZone<true>, DummyTransport>::new(our_basic_node_data_2, ())
            .build();

    let (p1, p2) = duplex(50_000);

    let (p1_receiver, p1_sender) = split(p1);
    let (p2_receiver, p2_sender) = split(p2);

    let p1_handshake_req = DoHandshakeRequest {
        addr: InternalPeerID::KnownAddr(TestNetZoneAddr(888)),
        peer_stream: FramedRead::new(p2_receiver, MoneroWireCodec::default()),
        peer_sink: FramedWrite::new(p2_sender, MoneroWireCodec::default()),
        direction: ConnectionDirection::Outbound,
        permit: None,
    };

    let p2_handshake_req = DoHandshakeRequest {
        addr: InternalPeerID::KnownAddr(TestNetZoneAddr(444)),
        peer_stream: FramedRead::new(p1_receiver, MoneroWireCodec::default()),
        peer_sink: FramedWrite::new(p1_sender, MoneroWireCodec::default()),
        direction: ConnectionDirection::Inbound,
        permit: None,
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

    let (res1, res2) = tokio::join!(p1, p2);
    res1.unwrap();
    res2.unwrap();
}

#[tokio::test]
async fn handshake_cuprate_to_monerod() {
    let monerod = monerod(["--fixed-difficulty=1", "--out-peers=0"]).await;

    let our_basic_node_data = BasicNodeData {
        my_port: 0,
        network_id: Network::Mainnet.network_id(),
        peer_id: 87980,
        support_flags: PeerSupportFlags::from(1_u32),
        rpc_port: 0,
        rpc_credits_per_hash: 0,
    };

    let handshaker = HandshakerBuilder::<ClearNet, Tcp>::new(our_basic_node_data, ()).build();

    let mut connector = Connector::new(handshaker);

    connector
        .ready()
        .await
        .unwrap()
        .call(ConnectRequest {
            addr: monerod.p2p_addr(),
            permit: None,
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn handshake_monerod_to_cuprate() {
    let our_basic_node_data = BasicNodeData {
        my_port: 18081,
        network_id: Network::Mainnet.network_id(),
        peer_id: 87980,
        support_flags: PeerSupportFlags::from(1_u32),
        rpc_port: 0,
        rpc_credits_per_hash: 0,
    };

    let mut handshaker = HandshakerBuilder::<ClearNet, Tcp>::new(our_basic_node_data, ()).build();

    let mut server_cfg = TcpServerConfig::default();
    server_cfg.ipv4 = Some("127.0.0.1".parse().unwrap());

    let mut listener = <Tcp as Transport<ClearNet>>::incoming_connection_listener(server_cfg)
        .await
        .unwrap();

    let _monerod = monerod(["--add-exclusive-node=127.0.0.1:18081"]).await;

    // Put a timeout on this just in case monerod doesn't make the connection to us.
    let next_connection_fut = timeout(Duration::from_secs(30), listener.next());

    if let Some(Ok((addr, stream, sink))) = next_connection_fut.await.unwrap() {
        handshaker
            .ready()
            .await
            .unwrap()
            .call(DoHandshakeRequest {
                addr: InternalPeerID::KnownAddr(addr.unwrap()), // This is clear net all addresses are known.
                peer_stream: stream,
                peer_sink: sink,
                direction: ConnectionDirection::Inbound,
                permit: None,
            })
            .await
            .unwrap();
    } else {
        panic!("Failed to receive connection from monerod.");
    };
}

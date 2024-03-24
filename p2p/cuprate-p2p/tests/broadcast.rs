use std::{
    collections::HashSet,
    pin::pin,
    sync::{Arc, Mutex},
    time::Duration,
};

use bytes::Bytes;
use futures::StreamExt;
use tokio::time::timeout;
use tower::{Service, ServiceExt};

use monero_p2p::{
    client::InternalPeerID, network_zones::ClearNet, BroadcastMessage, ConnectionDirection,
};

use cuprate_p2p::broadcast::{
    init_broadcast_channels, BroadcastConfig, BroadcastMessageStream, BroadcastRequest,
};

const TEST_CONFIG: BroadcastConfig = BroadcastConfig {
    diffusion_flush_average_seconds_outbound: 0.01,
    diffusion_flush_average_seconds_inbound: 0.01,
};

#[tokio::test]
async fn broadcast_send_recv() {
    let (mut svc, outbound_mkr, inbound_mkr) = init_broadcast_channels::<ClearNet>(&TEST_CONFIG);

    let inbound_stream_1 = inbound_mkr(InternalPeerID::Unknown(0));
    let inbound_stream_2 = inbound_mkr(InternalPeerID::Unknown(0));

    let outbound_stream_1 = outbound_mkr(InternalPeerID::Unknown(0));
    let outbound_stream_2 = outbound_mkr(InternalPeerID::Unknown(0));

    svc.ready()
        .await
        .unwrap()
        .call(BroadcastRequest::Block {
            block_bytes: Default::default(),
            current_blockchain_height: 1000,
        })
        .await
        .unwrap();

    let check = |srm: BroadcastMessageStream<_>| async {
        match pin!(srm).next().await.unwrap() {
            BroadcastMessage::NewFluffyBlock(block) => {
                assert_eq!(block.current_blockchain_height, 1000)
            }
            _ => unreachable!(),
        }
    };

    timeout(Duration::from_secs(5), async {
        check(inbound_stream_1).await;
        check(inbound_stream_2).await;
        check(outbound_stream_1).await;
        check(outbound_stream_2).await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn broadcasts_in_specific_direction() {
    timeout(Duration::from_secs(5), async {
        let (svc, outbound_mkr, inbound_mkr) = init_broadcast_channels::<ClearNet>(&TEST_CONFIG);

        let inbound_stream = inbound_mkr(InternalPeerID::Unknown(0));

        let outbound_stream = outbound_mkr(InternalPeerID::Unknown(0));

        let mut fut = svc.call_all(futures::stream::iter([
            BroadcastRequest::Transaction {
                tx_bytes: Bytes::copy_from_slice(&[1]),
                skip_peers: Arc::new(Mutex::new(HashSet::new())),
                direction: Some(ConnectionDirection::OutBound),
            },
            BroadcastRequest::Transaction {
                tx_bytes: Bytes::copy_from_slice(&[2]),
                skip_peers: Arc::new(Mutex::new(HashSet::new())),
                direction: Some(ConnectionDirection::OutBound),
            },
            BroadcastRequest::Transaction {
                tx_bytes: Bytes::copy_from_slice(&[3]),
                skip_peers: Arc::new(Mutex::new(HashSet::new())),
                direction: Some(ConnectionDirection::InBound),
            },
            BroadcastRequest::Transaction {
                tx_bytes: Bytes::copy_from_slice(&[4]),
                skip_peers: Arc::new(Mutex::new(HashSet::new())),
                direction: Some(ConnectionDirection::OutBound),
            },
        ]));
        while fut.next().await.is_some() {}

        let BroadcastMessage::NewTransaction(txs) = pin!(outbound_stream).next().await.unwrap()
        else {
            panic!()
        };

        assert_eq!(
            txs.txs,
            vec![
                Bytes::copy_from_slice(&[1]),
                Bytes::copy_from_slice(&[2]),
                Bytes::copy_from_slice(&[4])
            ]
        );

        assert!(txs.dandelionpp_fluff);

        let BroadcastMessage::NewTransaction(txs) = pin!(inbound_stream).next().await.unwrap()
        else {
            panic!()
        };

        assert_eq!(txs.txs, vec![Bytes::copy_from_slice(&[3])]);

        assert!(txs.dandelionpp_fluff);
    })
    .await
    .unwrap()
}

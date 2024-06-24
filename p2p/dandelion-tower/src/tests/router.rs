use std::time::Duration;

use tower::{Service, ServiceExt};

use crate::{DandelionConfig, DandelionRouteReq, DandelionRouter, Graph, TxState};

use super::*;

/// make sure the number of stemm peers is correct.
#[tokio::test]
async fn number_stems_correct() {
    let mut config = DandelionConfig {
        time_between_hop: Duration::from_millis(175),
        epoch_duration: Duration::from_secs(60_000),
        fluff_probability: 0.0, // we want to be in stem state
        graph: Graph::FourRegular,
    };

    let (broadcast_svc, _broadcast_rx) = mock_broadcast_svc();
    let (outbound_peer_svc, _outbound_rx) = mock_discover_svc();

    let mut router = DandelionRouter::new(broadcast_svc, outbound_peer_svc, config);

    const FROM_PEER: usize = 20;

    // send a request to make the generic bound inference work, without specifying types.
    router
        .ready()
        .await
        .unwrap()
        .call(DandelionRouteReq {
            tx: 0_usize,
            state: TxState::Stem { from: FROM_PEER },
        })
        .await
        .unwrap();

    assert_eq!(router.stem_peers.len(), 2); // Graph::FourRegular

    config.graph = Graph::Line;

    let (broadcast_svc, _broadcast_rx) = mock_broadcast_svc();
    let (outbound_peer_svc, _outbound_rx) = mock_discover_svc();

    let mut router = DandelionRouter::new(broadcast_svc, outbound_peer_svc, config);

    // send a request to make the generic bound inference work, without specifying types.
    router
        .ready()
        .await
        .unwrap()
        .call(DandelionRouteReq {
            tx: 0_usize,
            state: TxState::Stem { from: FROM_PEER },
        })
        .await
        .unwrap();

    assert_eq!(router.stem_peers.len(), 1); // Graph::Line
}

/// make sure a tx from the same peer goes to the same peer.
#[tokio::test]
async fn routes_consistent() {
    let config = DandelionConfig {
        time_between_hop: Duration::from_millis(175),
        epoch_duration: Duration::from_secs(60_000),
        fluff_probability: 0.0, // we want this test to always stem
        graph: Graph::FourRegular,
    };

    let (broadcast_svc, mut broadcast_rx) = mock_broadcast_svc();
    let (outbound_peer_svc, mut outbound_rx) = mock_discover_svc();

    let mut router = DandelionRouter::new(broadcast_svc, outbound_peer_svc, config);

    const FROM_PEER: usize = 20;

    // The router will panic if it attempts to flush.
    broadcast_rx.close();

    for _ in 0..30 {
        router
            .ready()
            .await
            .unwrap()
            .call(DandelionRouteReq {
                tx: 0_usize,
                state: TxState::Stem { from: FROM_PEER },
            })
            .await
            .unwrap();
    }

    let mut stem_peer = None;
    let mut total_txs = 0;

    while let Ok((peer_id, _)) = outbound_rx.try_recv() {
        let stem_peer = stem_peer.get_or_insert(peer_id);
        // make sure all peer ids are the same (so the same svc got all txs).
        assert_eq!(*stem_peer, peer_id);

        total_txs += 1;
    }

    assert_eq!(total_txs, 30);
}

/// make sure local txs always stem - even in fluff state.
#[tokio::test]
async fn local_always_stem() {
    let config = DandelionConfig {
        time_between_hop: Duration::from_millis(175),
        epoch_duration: Duration::from_secs(60_000),
        fluff_probability: 1.0, // we want this test to always fluff
        graph: Graph::FourRegular,
    };

    let (broadcast_svc, mut broadcast_rx) = mock_broadcast_svc();
    let (outbound_peer_svc, mut outbound_rx) = mock_discover_svc();

    let mut router = DandelionRouter::new(broadcast_svc, outbound_peer_svc, config);

    // The router will panic if it attempts to flush.
    broadcast_rx.close();

    for _ in 0..30 {
        router
            .ready()
            .await
            .unwrap()
            .call(DandelionRouteReq {
                tx: 0_usize,
                state: TxState::Local,
            })
            .await
            .unwrap();
    }

    let mut stem_peer = None;
    let mut total_txs = 0;

    while let Ok((peer_id, _)) = outbound_rx.try_recv() {
        let stem_peer = stem_peer.get_or_insert(peer_id);
        // make sure all peer ids are the same (so the same svc got all txs).
        assert_eq!(*stem_peer, peer_id);

        total_txs += 1;
    }

    assert_eq!(total_txs, 30);
}

/// make sure local txs always stem - even in fluff state.
#[tokio::test]
async fn stem_txs_fluff_in_state_fluff() {
    let config = DandelionConfig {
        time_between_hop: Duration::from_millis(175),
        epoch_duration: Duration::from_secs(60_000),
        fluff_probability: 1.0, // we want this test to always fluff
        graph: Graph::FourRegular,
    };

    let (broadcast_svc, mut broadcast_rx) = mock_broadcast_svc();
    let (outbound_peer_svc, mut outbound_rx) = mock_discover_svc();

    let mut router = DandelionRouter::new(broadcast_svc, outbound_peer_svc, config);

    const FROM_PEER: usize = 20;

    // The router will panic if it attempts to stem.
    outbound_rx.close();

    for _ in 0..30 {
        router
            .ready()
            .await
            .unwrap()
            .call(DandelionRouteReq {
                tx: 0_usize,
                state: TxState::Stem { from: FROM_PEER },
            })
            .await
            .unwrap();
    }

    let mut total_txs = 0;

    while broadcast_rx.try_recv().is_ok() {
        total_txs += 1;
    }

    assert_eq!(total_txs, 30);
}

/// make sure we get all txs sent to the router out in a stem or a fluff.
#[tokio::test]
async fn random_routing() {
    let config = DandelionConfig {
        time_between_hop: Duration::from_millis(175),
        epoch_duration: Duration::from_secs(0), // make every poll ready change state
        fluff_probability: 0.2,
        graph: Graph::FourRegular,
    };

    let (broadcast_svc, mut broadcast_rx) = mock_broadcast_svc();
    let (outbound_peer_svc, mut outbound_rx) = mock_discover_svc();

    let mut router = DandelionRouter::new(broadcast_svc, outbound_peer_svc, config);

    for _ in 0..3000 {
        router
            .ready()
            .await
            .unwrap()
            .call(DandelionRouteReq {
                tx: 0_usize,
                state: TxState::Stem {
                    from: rand::random(),
                },
            })
            .await
            .unwrap();
    }

    let mut total_txs = 0;

    while broadcast_rx.try_recv().is_ok() {
        total_txs += 1;
    }

    while outbound_rx.try_recv().is_ok() {
        total_txs += 1;
    }

    assert_eq!(total_txs, 3000);
}

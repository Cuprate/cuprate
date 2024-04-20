use std::time::Duration;

use tower::{Service, ServiceExt};

use dandelion_tower::{DandelionConfig, DandelionRouteReq, DandelionRouterBuilder, Graph, TxState};

mod common;

#[tokio::test]
async fn routes_constant() {
    let config = DandelionConfig {
        time_between_hop: Duration::from_millis(175),
        epoch_duration: Duration::from_secs(600_000_000),
        fluff_probability: 0.0,
        graph: Graph::FourRegular,
    };

    let (broadcast_svc, mut broadcast_rx) = common::mock_broadcast_svc();
    let (outbound_peer_svc, mut outbound_rx) = common::mock_discover_svc();

    let mut router = DandelionRouterBuilder::default()
        .with_config(config)
        .with_broadcast_svc(broadcast_svc)
        .with_outbound_peer_discover(outbound_peer_svc)
        .build();

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

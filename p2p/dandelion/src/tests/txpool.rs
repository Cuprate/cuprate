use std::time::Duration;

use crate::{
    txpool::{DandelionPoolRequest, DandelionPoolServiceBuilder},
    DandelionConfig, DandelionRouterBuilder, Graph, TxState,
};

use super::*;

#[tokio::test]
async fn basic_functionality() {
    let config = DandelionConfig {
        time_between_hop: Duration::from_millis(175),
        epoch_duration: Duration::from_secs(0), // make every poll ready change state
        fluff_probability: 0.2,
        graph: Graph::FourRegular,
    };

    let (broadcast_svc, mut broadcast_rx) = mock_broadcast_svc();
    let (outbound_peer_svc, _outbound_rx) = mock_discover_svc();

    let router = DandelionRouterBuilder::default()
        .with_config(config)
        .with_broadcast_svc(broadcast_svc)
        .with_outbound_peer_discover(outbound_peer_svc)
        .build();

    let (pool_svc, pool) = mock_in_memory_backing_pool();

    let mut pool_svc = DandelionPoolServiceBuilder::default()
        .with_config(config)
        .with_backing_pool(pool_svc)
        .with_router(router)
        .spawn(15);

    pool_svc
        .ready()
        .await
        .unwrap()
        .call(DandelionPoolRequest::IncomingTx {
            tx: 0_usize,
            tx_id: 1_usize,
            tx_state: TxState::Fluff,
        })
        .await
        .unwrap();

    assert!(pool.lock().unwrap().contains_key(&1));
    assert!(broadcast_rx.try_recv().is_ok())
}

use std::time::Duration;

use super::*;
use crate::{
    DandelionConfig, DandelionRouter, Graph, TxState,
    pool::{IncomingTx, start_dandelion_pool_manager},
};

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

    let router = DandelionRouter::new(broadcast_svc, outbound_peer_svc, config);

    let (pool_svc, _pool) = mock_in_memory_backing_pool();

    let mut pool_svc = start_dandelion_pool_manager(15, router, pool_svc, config);

    pool_svc
        .ready()
        .await
        .unwrap()
        .call(IncomingTx {
            tx: 0_usize,
            tx_id: 1_usize,
            routing_state: TxState::Fluff,
        })
        .await
        .unwrap();

    // TODO: the DandelionPoolManager doesn't handle adding txs to the pool, add more tests here to test
    // all functionality.
    //assert!(pool.lock().unwrap().contains_key(&1));
    assert!(broadcast_rx.try_recv().is_ok());
}

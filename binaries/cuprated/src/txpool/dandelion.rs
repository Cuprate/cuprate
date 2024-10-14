use std::time::Duration;

use bytes::Bytes;
use cuprate_dandelion_tower::pool::DandelionPoolService;
use cuprate_dandelion_tower::{DandelionConfig, DandelionRouter, Graph};
use cuprate_p2p::NetworkInterface;
use cuprate_p2p_core::ClearNet;
use cuprate_txpool::service::{TxpoolReadHandle, TxpoolWriteHandle};
use cuprate_wire::NetworkAddress;

mod diffuse_service;
mod stem_service;
mod tx_store;

#[derive(Clone)]
pub struct DandelionTx(Bytes);

type TxId = [u8; 32];

const DANDELION_CONFIG: DandelionConfig = DandelionConfig {
    time_between_hop: Duration::from_millis(175),
    epoch_duration: Duration::from_secs(10 * 60),
    fluff_probability: 0.12,
    graph: Graph::FourRegular,
};

type ConcreteDandelionRouter = DandelionRouter<
    stem_service::OutboundPeerStream,
    diffuse_service::DiffuseService,
    NetworkAddress,
    stem_service::StemPeerService<ClearNet>,
    DandelionTx,
>;

pub fn start_dandelion_pool_manager(
    router: ConcreteDandelionRouter,
    txpool_read_handle: TxpoolReadHandle,
    txpool_write_handle: TxpoolWriteHandle,
) -> DandelionPoolService<DandelionTx, TxId, NetworkAddress> {
    cuprate_dandelion_tower::pool::start_dandelion_pool_manager(
        12,
        router,
        tx_store::TxStoreService {
            txpool_read_handle,
            txpool_write_handle,
        },
        DANDELION_CONFIG,
    )
}

pub fn dandelion_router(clear_net: NetworkInterface<ClearNet>) -> ConcreteDandelionRouter {
    DandelionRouter::new(
        diffuse_service::DiffuseService {
            clear_net_broadcast_service: clear_net.broadcast_svc(),
        },
        stem_service::OutboundPeerStream { clear_net },
        DANDELION_CONFIG,
    )
}

use bytes::Bytes;
use cuprate_dandelion_tower::pool::DandelionPoolService;
use cuprate_dandelion_tower::{DandelionConfig, DandelionRouter};
use cuprate_p2p::NetworkInterface;
use cuprate_p2p_core::ClearNet;
use cuprate_txpool::service::{TxpoolReadHandle, TxpoolWriteHandle};
use cuprate_wire::NetworkAddress;

mod diffuse_service;
mod stem_service;
mod tx_store;

#[derive(Clone)]
struct DandelionTx(Bytes);

type TxId = [u8; 32];

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
        DandelionConfig {
            time_between_hop: Default::default(),
            epoch_duration: Default::default(),
            fluff_probability: 0.0,
            graph: Default::default(),
        },
    )
}

pub fn dandelion_router(clear_net: NetworkInterface<ClearNet>) -> ConcreteDandelionRouter {
    DandelionRouter::new(
        diffuse_service::DiffuseService {
            clear_net_broadcast_service: clear_net.broadcast_svc(),
        },
        stem_service::OutboundPeerStream {
            clear_net: clear_net.clone(),
        },
        DandelionConfig {
            time_between_hop: Default::default(),
            epoch_duration: Default::default(),
            fluff_probability: 0.0,
            graph: Default::default(),
        },
    )
}

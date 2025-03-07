use std::time::Duration;

use cuprate_dandelion_tower::{
    DandelionConfig, DandelionRouter, Graph, pool::DandelionPoolService,
};
use cuprate_p2p::NetworkInterface;
use cuprate_p2p_core::ClearNet;
use cuprate_txpool::service::{TxpoolReadHandle, TxpoolWriteHandle};

use crate::{
    p2p::CrossNetworkInternalPeerId,
    txpool::incoming_tx::{DandelionTx, TxId},
};

mod diffuse_service;
mod stem_service;
mod tx_store;

/// The configuration used for [`cuprate_dandelion_tower`].
///
/// TODO: should we expose this to users of cuprated? probably not.
const DANDELION_CONFIG: DandelionConfig = DandelionConfig {
    time_between_hop: Duration::from_millis(175),
    epoch_duration: Duration::from_secs(10 * 60),
    fluff_probability: 0.12,
    graph: Graph::FourRegular,
};

/// A [`DandelionRouter`] with all generic types defined.
type ConcreteDandelionRouter = DandelionRouter<
    stem_service::OutboundPeerStream,
    diffuse_service::DiffuseService,
    CrossNetworkInternalPeerId,
    stem_service::StemPeerService<ClearNet>,
    DandelionTx,
>;

/// Starts the dandelion pool manager task and returns a handle to send txs to broadcast.
pub fn start_dandelion_pool_manager(
    router: ConcreteDandelionRouter,
    txpool_read_handle: TxpoolReadHandle,
    txpool_write_handle: TxpoolWriteHandle,
) -> DandelionPoolService<DandelionTx, TxId, CrossNetworkInternalPeerId> {
    cuprate_dandelion_tower::pool::start_dandelion_pool_manager(
        // TODO: make this constant configurable?
        32,
        router,
        tx_store::TxStoreService {
            txpool_read_handle,
            txpool_write_handle,
        },
        DANDELION_CONFIG,
    )
}

/// Creates a [`DandelionRouter`] from a [`NetworkInterface`].
pub fn dandelion_router(clear_net: NetworkInterface<ClearNet>) -> ConcreteDandelionRouter {
    DandelionRouter::new(
        diffuse_service::DiffuseService {
            clear_net_broadcast_service: clear_net.broadcast_svc(),
        },
        stem_service::OutboundPeerStream::new(clear_net),
        DANDELION_CONFIG,
    )
}

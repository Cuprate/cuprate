use bytes::Bytes;
use cuprate_dandelion_tower::{DandelionConfig, DandelionRouter};
use cuprate_p2p::NetworkInterface;
use cuprate_p2p_core::ClearNet;
use cuprate_wire::NetworkAddress;

mod diffuse_service;
mod stem_service;
mod tx_store;

struct DandelionTx(Bytes);

type TxId = [u8; 32];

pub fn start_dandelion_router(
    clear_net: NetworkInterface<ClearNet>,
) -> DandelionRouter<
    stem_service::OutboundPeerStream,
    diffuse_service::DiffuseService,
    NetworkAddress,
    stem_service::StemPeerService<ClearNet>,
    DandelionTx,
> {
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

use std::{
    task::{ready, Poll},
    time::Duration,
};

use futures::{future::BoxFuture, FutureExt, TryFutureExt};
use tower::{Service, ServiceExt};

use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::PollSender;

use cuprate_dandelion_tower::{
    pool::DandelionPoolService, traits::StemRequest, DandelionConfig, DandelionRouteReq,
    DandelionRouter, DandelionRouterError, Graph, State, TxState,
};
use cuprate_p2p::NetworkInterface;
use cuprate_p2p_core::{client::InternalPeerID, ClearNet, NetworkZone, Tor};
use cuprate_txpool::service::{TxpoolReadHandle, TxpoolWriteHandle};

use crate::{
    p2p::CrossNetworkInternalPeerId,
    txpool::incoming_tx::{DandelionTx, TxId},
};

mod anon_net_service;
mod diffuse_service;
mod stem_service;
mod tx_store;

pub use anon_net_service::AnonTxService;
pub use diffuse_service::DiffuseService;

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
pub(super) type ConcreteDandelionRouter<Z> = DandelionRouter<
    stem_service::OutboundPeerStream<Z>,
    DiffuseService<Z>,
    CrossNetworkInternalPeerId,
    stem_service::StemPeerService<Z>,
    DandelionTx,
>;

/// The dandelion router used to send transactions to the network.
pub(super) struct MainDandelionRouter {
    clearnet_router: ConcreteDandelionRouter<ClearNet>,
    tor_router: Option<AnonTxService<Tor>>,
    tor_net_rx: Option<oneshot::Receiver<NetworkInterface<Tor>>>,
}

impl MainDandelionRouter {
    pub const fn new(
        clearnet_router: ConcreteDandelionRouter<ClearNet>,
        tor_net_rx: Option<oneshot::Receiver<NetworkInterface<Tor>>>,
    ) -> Self {
        Self {
            clearnet_router,
            tor_router: None,
            tor_net_rx,
        }
    }
}

impl Service<DandelionRouteReq<DandelionTx, CrossNetworkInternalPeerId>> for MainDandelionRouter {
    type Response = State;
    type Error = DandelionRouterError;
    type Future = BoxFuture<'static, Result<State, DandelionRouterError>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Check if Tor router has arrived
        if self.tor_router.is_none() {
            if let Some(rx) = self.tor_net_rx.as_mut() {
                match rx.try_recv() {
                    Ok(interface) => {
                        self.tor_router = Some(AnonTxService::new(interface));
                        self.tor_net_rx = None;
                        tracing::info!("Tor P2P zone is now available.");
                    }
                    Err(oneshot::error::TryRecvError::Empty) => {}
                    Err(oneshot::error::TryRecvError::Closed) => {
                        self.tor_net_rx = None;
                    }
                }
            }
        }

        if let Some(tor_router) = self.tor_router.as_mut() {
            ready!(tor_router.poll_ready(cx))?;
        }

        self.clearnet_router.poll_ready(cx)
    }

    fn call(
        &mut self,
        req: DandelionRouteReq<DandelionTx, CrossNetworkInternalPeerId>,
    ) -> Self::Future {
        // TODO: is this the best way to use anonymity networks?
        if req.state == TxState::Local {
            if let Some(tor_router) = self.tor_router.as_mut() {
                if let Some(mut peer) = tor_router.peer.take() {
                    tracing::debug!("routing tx over Tor");
                    return peer
                        .call(StemRequest(req.tx))
                        .map_ok(|_| State::Stem)
                        .map_err(DandelionRouterError::PeerError)
                        .boxed();
                }

                tracing::warn!(
                    "failed to route tx over Tor, no connections, falling back to Clearnet"
                );
            }
        }

        self.clearnet_router.call(req)
    }
}

/// Starts the dandelion pool manager task and returns a handle to send txs to broadcast.
pub fn start_dandelion_pool_manager(
    router: MainDandelionRouter,
    txpool_read_handle: TxpoolReadHandle,
    promote_tx: mpsc::UnboundedSender<[u8; 32]>,
) -> DandelionPoolService<DandelionTx, TxId, CrossNetworkInternalPeerId> {
    cuprate_dandelion_tower::pool::start_dandelion_pool_manager(
        // TODO: make this constant configurable?
        32,
        router,
        tx_store::TxStoreService {
            txpool_read_handle,
            promote_tx,
        },
        DANDELION_CONFIG,
    )
}

/// Creates a [`DandelionRouter`] from a [`NetworkInterface`].
pub fn dandelion_router<Z: NetworkZone>(
    network_interface: NetworkInterface<Z>,
) -> ConcreteDandelionRouter<Z>
where
    InternalPeerID<Z::Addr>: Into<CrossNetworkInternalPeerId>,
{
    DandelionRouter::new(
        DiffuseService {
            clear_net_broadcast_service: network_interface.broadcast_svc(),
        },
        stem_service::OutboundPeerStream::<Z>::new(network_interface),
        DANDELION_CONFIG,
    )
}

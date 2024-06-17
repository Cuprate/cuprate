use std::future::{ready, Ready};
use std::task::{Context, Poll};
use tower::Service;

use monero_wire::{BasicNodeData, CoreSyncData};

use crate::services::{
    CoreSyncDataRequest, CoreSyncDataResponse, PeerSyncRequest, PeerSyncResponse,
};
use crate::{NetworkZone, PeerRequest, PeerResponse};

pub struct HandshakerBuilder<
    AdrBook,
    CSync = DummyCoreSyncSvc,
    PSync = DummyPeerSyncSvc,
    ReqHdlr = DummyPeerRequestHdlr,
    BrdcstStrmMkr = (),
> {
    /// The address book service.
    address_book: AdrBook,
    /// The core sync data service.
    core_sync_svc: Option<CSync>,
    /// The peer sync service.
    peer_sync_svc: Option<PSync>,
    /// The peer request handler service.
    peer_request_svc: Option<ReqHdlr>,

    /// Our [`BasicNodeData`]
    our_basic_node_data: BasicNodeData,

    /// A function that returns a stream that will give items to be broadcast by a connection.
    broadcast_stream_maker: Option<BrdcstStrmMkr>,

    silence_warnings: bool,
}

impl<AdrBook, CSync, PSync, ReqHdlr, BrdcstStrmMkr>
    HandshakerBuilder<AdrBook, CSync, PSync, ReqHdlr, BrdcstStrmMkr>
{
}

pub struct DummyPeerRequestHdlr;

impl Service<PeerRequest> for DummyPeerRequestHdlr {
    type Response = PeerResponse;
    type Error = tower::BoxError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: PeerRequest) -> Self::Future {
        ready(Ok(PeerResponse::NA))
    }
}

pub struct DummyPeerSyncSvc;

impl<N: NetworkZone> Service<PeerSyncRequest<N>> for DummyPeerSyncSvc {
    type Response = PeerSyncResponse<N>;
    type Error = tower::BoxError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: PeerSyncRequest<N>) -> Self::Future {
        ready(Ok(match req {
            PeerSyncRequest::PeersToSyncFrom { .. } => PeerSyncResponse::PeersToSyncFrom(vec![]),
            PeerSyncRequest::IncomingCoreSyncData(_, _, _) => PeerSyncResponse::Ok,
        }))
    }
}

pub struct DummyCoreSyncSvc(CoreSyncData);

impl DummyCoreSyncSvc {
    pub fn static_mainnet_genesis() -> DummyCoreSyncSvc {
        DummyCoreSyncSvc(CoreSyncData {
            cumulative_difficulty: 1,
            cumulative_difficulty_top64: 0,
            current_height: 1,
            pruning_seed: 0,
            top_id: hex_literal::hex!(
                "418015bb9ae982a1975da7d79277c2705727a56894ba0fb246adaabb1f4632e3"
            ),
            top_version: 1,
        })
    }

    pub fn static_testnet_genesis() -> DummyCoreSyncSvc {
        DummyCoreSyncSvc(CoreSyncData {
            cumulative_difficulty: 1,
            cumulative_difficulty_top64: 0,
            current_height: 1,
            pruning_seed: 0,
            top_id: hex_literal::hex!(
                "48ca7cd3c8de5b6a4d53d2861fbdaedca141553559f9be9520068053cda8430b"
            ),
            top_version: 1,
        })
    }

    pub fn static_stagenet_genesis() -> DummyCoreSyncSvc {
        DummyCoreSyncSvc(CoreSyncData {
            cumulative_difficulty: 1,
            cumulative_difficulty_top64: 0,
            current_height: 1,
            pruning_seed: 0,
            top_id: hex_literal::hex!(
                "76ee3cc98646292206cd3e86f74d88b4dcc1d937088645e9b0cbca84b7ce74eb"
            ),
            top_version: 1,
        })
    }

    pub fn static_custom(data: CoreSyncData) -> DummyCoreSyncSvc {
        DummyCoreSyncSvc(data)
    }
}

impl Service<CoreSyncDataRequest> for DummyCoreSyncSvc {
    type Response = CoreSyncDataResponse;
    type Error = tower::BoxError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: CoreSyncDataRequest) -> Self::Future {
        ready(Ok(CoreSyncDataResponse(self.0.clone())))
    }
}

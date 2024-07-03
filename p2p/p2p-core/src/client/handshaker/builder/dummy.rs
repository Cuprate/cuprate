use std::{
    future::{ready, Ready},
    task::{Context, Poll},
};

use tower::Service;

use cuprate_wire::CoreSyncData;

use crate::{
    services::{
        AddressBookRequest, AddressBookResponse, CoreSyncDataRequest, CoreSyncDataResponse,
        PeerSyncRequest, PeerSyncResponse,
    },
    NetworkZone, ProtocolRequest, ProtocolResponse,
};

/// A dummy peer sync service, that doesn't actually keep track of peers sync states.
#[derive(Debug, Clone)]
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

/// A dummy core sync service that just returns static [`CoreSyncData`].
#[derive(Debug, Clone)]
pub struct DummyCoreSyncSvc(CoreSyncData);

impl DummyCoreSyncSvc {
    /// Returns a [`DummyCoreSyncSvc`] that will just return the mainnet genesis [`CoreSyncData`].
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

    /// Returns a [`DummyCoreSyncSvc`] that will just return the testnet genesis [`CoreSyncData`].
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

    /// Returns a [`DummyCoreSyncSvc`] that will just return the stagenet genesis [`CoreSyncData`].
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

    /// Returns a [`DummyCoreSyncSvc`] that will return the provided [`CoreSyncData`].
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

/// A dummy address book that doesn't actually keep track of peers.
#[derive(Debug, Clone)]
pub struct DummyAddressBook;

impl<N: NetworkZone> Service<AddressBookRequest<N>> for DummyAddressBook {
    type Response = AddressBookResponse<N>;
    type Error = tower::BoxError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: AddressBookRequest<N>) -> Self::Future {
        ready(Ok(match req {
            AddressBookRequest::GetWhitePeers(_) => AddressBookResponse::Peers(vec![]),
            AddressBookRequest::TakeRandomGrayPeer { .. }
            | AddressBookRequest::TakeRandomPeer { .. }
            | AddressBookRequest::TakeRandomWhitePeer { .. } => {
                return ready(Err("dummy address book does not hold peers".into()));
            }
            AddressBookRequest::NewConnection { .. } | AddressBookRequest::IncomingPeerList(_) => {
                AddressBookResponse::Ok
            }
            AddressBookRequest::IsPeerBanned(_) => AddressBookResponse::IsPeerBanned(false),
        }))
    }
}

/// A dummy protocol request handler.
#[derive(Debug, Clone)]
pub struct DummyProtocolRequestHandler;

impl Service<ProtocolRequest> for DummyProtocolRequestHandler {
    type Response = ProtocolResponse;
    type Error = tower::BoxError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: ProtocolRequest) -> Self::Future {
        ready(Ok(ProtocolResponse::NA))
    }
}

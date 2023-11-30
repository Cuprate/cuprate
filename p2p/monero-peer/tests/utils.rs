use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use futures::FutureExt;
use tower::Service;

use monero_peer::{
    services::{
        AddressBookRequest, AddressBookResponse, CoreSyncDataRequest, CoreSyncDataResponse,
    },
    NetworkZone, PeerRequest, PeerResponse,
};

#[derive(Clone)]
pub struct DummyAddressBook;

impl<Z: NetworkZone> Service<AddressBookRequest<Z>> for DummyAddressBook {
    type Response = AddressBookResponse<Z>;
    type Error = tower::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: AddressBookRequest<Z>) -> Self::Future {
        async move {
            Ok(match req {
                AddressBookRequest::GetPeers(_) => AddressBookResponse::Peers(vec![]),
                _ => AddressBookResponse::Ok,
            })
        }
        .boxed()
    }
}

#[derive(Clone)]
pub struct DummyCoreSyncSvc;

impl Service<CoreSyncDataRequest> for DummyCoreSyncSvc {
    type Response = CoreSyncDataResponse;
    type Error = tower::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: CoreSyncDataRequest) -> Self::Future {
        async move {
            match req {
                CoreSyncDataRequest::Ours => {
                    Ok(CoreSyncDataResponse::Ours(monero_wire::CoreSyncData {
                        cumulative_difficulty: 1,
                        cumulative_difficulty_top64: 0,
                        current_height: 1,
                        pruning_seed: 0,
                        top_id: hex::decode(
                            "418015bb9ae982a1975da7d79277c2705727a56894ba0fb246adaabb1f4632e3",
                        )
                        .unwrap()
                        .try_into()
                        .unwrap(),
                        top_version: 1,
                    }))
                }
                CoreSyncDataRequest::HandleIncoming(_) => Ok(CoreSyncDataResponse::Ok),
            }
        }
        .boxed()
    }
}

#[derive(Clone)]
pub struct DummyPeerRequestHandlerSvc;

impl Service<PeerRequest> for DummyPeerRequestHandlerSvc {
    type Response = PeerResponse;
    type Error = tower::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn call(&mut self, req: PeerRequest) -> Self::Future {
        todo!()
    }
}

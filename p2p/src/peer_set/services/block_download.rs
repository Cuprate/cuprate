use futures::{FutureExt, Sink};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::sync::Mutex;
use tower::discover::Discover;
use tower::BoxError;

use cuprate_common::pruning;
use monero_wire::messages::GetObjectsResponse;
use monero_wire::NetworkAddress;

use crate::peer::LoadTrackedClient;
use crate::peer_set::set::PeerSet;

pub enum BlockGetterRequest {
    /// A request for blocks, used when syncing.
    ///
    /// start_height is used to determine the peer for the next request,
    /// you should use [`BlockGetterRequest::SetHeight`] before calling
    /// this for the first time.
    GetBlocks {
        blocks: Vec<[u8; 32]>,
        pruned: bool,
        start_height: u64,
        target_height: u64,
    },
    SetHeight {
        our_height: u64,
        target: u64,
    },
}

pub enum BlockGetterResponse {
    Blocks(GetObjectsResponse),
    HeightSet,
}

pub struct BlockGetterService<D>
where
    D: Discover<Key = NetworkAddress, Service = LoadTrackedClient> + Unpin,
    D::Error: Into<BoxError>,
{
    peer_set: Arc<Mutex<PeerSet<D>>>,
    next_start_height: Option<u64>,
    p2c_peer: Option<D::Service>,
}

impl<D> tower::Service<BlockGetterRequest> for BlockGetterService<D>
where
    D: Discover<Key = NetworkAddress, Service = LoadTrackedClient> + Unpin,
    D::Error: Into<BoxError>,
{
    type Response = BlockGetterResponse;
    type Error = BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.next_start_height {
            // If we don't know the next batch start height we must have not received
            // any requests yet. The first request has to be [`SetHeight`] so thats
            // what the next request will be.
            None => Poll::Ready(Ok(())),
            Some(height) => {
                let mutex = self.peer_set.clone();
                match Box::pin(mutex.lock()).poll_unpin(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(mut peer_set) => {
                        peer_set.poll_all(cx)?;
                        let p2c_peer = match peer_set.preselect_p2c_peer_with_full_block(height) {
                            None => {
                                peer_set.demand_more_peers();
                                return Poll::Pending;
                            }
                            Some(peer) => peer,
                        };
                        self.p2c_peer = peer_set.take_ready_service(&p2c_peer);
                        Poll::Ready(Ok(()))
                    }
                }
            }
        }
    }

    fn call(&mut self, req: BlockGetterRequest) -> Self::Future {
        todo!()
    }
}

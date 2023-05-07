use futures::{FutureExt, Sink};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::sync::Mutex;
use tower::discover::Discover;
use tower::BoxError;

use monero_wire::messages::GetObjectsRequest;
use monero_wire::messages::{GetObjectsResponse, MessageNotification};
use monero_wire::{Message, NetworkAddress};

use crate::peer::LoadTrackedClient;
use crate::peer_set::set::PeerSet;
use crate::protocol::InternalMessageResponse;

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
    },
    SetHeight(u64),
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
    p2c_peer: Option<(D::Key, D::Service)>,
}

impl<D> tower::Service<BlockGetterRequest> for BlockGetterService<D>
where
    D: Discover<Key = NetworkAddress, Service = LoadTrackedClient> + Unpin + Send,
    D::Error: Into<BoxError>,
{
    type Response = BlockGetterResponse;
    type Error = BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let span = tracing::trace_span!(parent: &tracing::span::Span::current(), "BlockGetter");
        match self.next_start_height {
            // If we don't know the next batch start height we must have not received
            // any requests yet. The first request has to be [`SetHeight`] so thats
            // what the next request will be.
            None => {
                tracing::trace!(parent: &span, "next height not known");
                Poll::Ready(Ok(()))
            }
            Some(height) => {
                tracing::trace!(parent: &span, next_height = height);

                let mut peer_no_longer_ready = false;

                if let Some((addr, svc)) = &mut self.p2c_peer {
                    tracing::trace!(parent: &span,  preselected_peer = ?addr);
                    match svc.poll_ready(cx) {
                        Poll::Ready(Ok(())) => {
                            tracing::trace!(
                                parent: &span,
                                "Pre-selected peer still ready, keeping it selected"
                            );
                            return Poll::Ready(Ok(()));
                        }
                        Poll::Pending => {
                            tracing::trace!(
                                "preselected service is no longer ready, moving to unready list"
                            );
                            peer_no_longer_ready = true;
                        }
                        Poll::Ready(Err(e)) => {
                            tracing::trace!(parent: &span, %e, "preselected service failed, dropping it");
                            self.p2c_peer = None;
                        }
                    };
                }

                tracing::trace!(
                    parent: &span,
                    "preselected service was not ready, preselecting another ready service"
                );

                let mutex = self.peer_set.clone();
                match Box::pin(mutex.lock()).poll_unpin(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(mut peer_set) => {
                        peer_set.poll_all(cx)?;

                        if peer_no_longer_ready {
                            let (key, svc) = self
                                .p2c_peer
                                .expect("Peer must exist for it to not be ready");
                            peer_set.push_unready(key, svc);
                        }

                        let p2c_peer = match peer_set.preselect_p2c_peer_with_full_block(height) {
                            None => {
                                tracing::trace!(
                                    parent: &span,
                                    "no ready services, sending demand signal"
                                );
                                peer_set.demand_more_peers();
                                return Poll::Pending;
                            }
                            Some(peer) => {
                                tracing::trace!(parent: &span,  preselected_peer = ?peer);
                                peer
                            }
                        };
                        self.p2c_peer = peer_set
                            .take_ready_service(&p2c_peer)
                            .and_then(|svc| Some((p2c_peer, svc)));
                        Poll::Ready(Ok(()))
                    }
                }
            }
        }
    }

    fn call(&mut self, req: BlockGetterRequest) -> Self::Future {
        match req {
            BlockGetterRequest::SetHeight(height) => {
                self.next_start_height = Some(height);
                async { Ok(BlockGetterResponse::HeightSet) }.boxed()
            }
            BlockGetterRequest::GetBlocks {
                blocks,
                pruned,
                start_height,
            } => {
                self.next_start_height = Some(start_height + blocks.len() as u64);
                let obj_req = GetObjectsRequest { blocks, pruned };

                let peer_set = self.peer_set.clone();
                let (addr, mut svc) = std::mem::replace(&mut self.p2c_peer, None).expect(
                    "A peer is always selected in poll_ready and poll_ready must be called first",
                );

                async move {
                    let fut = svc.call(obj_req.into());

                    let mut set = peer_set.lock().await;
                    set.push_unready(addr, svc);
                    fut.await.map(|res| {
                        let InternalMessageResponse::GetObjectsResponse(res) = res else {
                            unreachable!("Peer connection must return correct response")
                        };
                        BlockGetterResponse::Blocks(res)
                    })
                }
                .boxed()
            }
        }
    }
}

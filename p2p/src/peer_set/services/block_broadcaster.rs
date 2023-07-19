// TODO: Investigate tor/i2p block broadcasting; should we do it? randomise delay?
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::stream::FuturesOrdered;
use futures::{FutureExt, StreamExt};
use tokio::sync::Mutex;
use tower::discover::Discover;
use tower::BoxError;

use monero_wire::messages::common::{BlockCompleteEntry, TransactionBlobs};
use monero_wire::messages::{NewBlock, NewFluffyBlock};
use monero_wire::{NetworkAddress, PeerID};

use crate::peer::LoadTrackedClient;
use crate::peer_set::set::PeerSet;

pub enum BlockBroadCasterRequest {
    /// A request to broadcast a block to all ready peers, Cuprate
    /// only supports broadcasting by fluffy blocks.
    BroadCastBlock { block: Vec<u8>, block_height: u64 },
}

pub enum BlockBroadCasterResponse {
    BlockBroadCasted,
}

pub struct BlockBroadCaster<D>
where
    D: Discover<Key = PeerID, Service = LoadTrackedClient> + Unpin,
    D::Error: Into<BoxError>,
{
    peer_set: std::sync::Arc<Mutex<PeerSet<D>>>,
    /// The proportion of peers that need to be ready for `poll_ready` to return ready.
    ///
    /// monerod will remove peers that do not broadcast every block to it, this is a problem
    /// for us as we need peers to be ready for us to broadcast to them so we compromise and
    /// only broadcast to ready peers and take the hit on the other peers.
    wanted_ready_peers: f64,
}

impl<D> tower::Service<BlockBroadCasterRequest> for BlockBroadCaster<D>
where
    D: Discover<Key = PeerID, Service = LoadTrackedClient> + Unpin + Send + 'static,
    D::Error: Into<BoxError>,
{
    type Response = BlockBroadCasterResponse;
    type Error = BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mutex = self.peer_set.clone();
        let ret = match Box::pin(mutex.lock()).poll_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(mut peer_set) => {
                peer_set.poll_all(cx)?;
                peer_set.poll_ready(cx);
                if self.wanted_ready_peers <= peer_set.proportion_ready() {
                    Poll::Ready(Ok(()))
                } else {
                    Poll::Pending
                }
            }
        };
        ret
    }

    fn call(&mut self, req: BlockBroadCasterRequest) -> Self::Future {
        match req {
            BlockBroadCasterRequest::BroadCastBlock {
                block,
                block_height,
            } => {
                let empty_txs = TransactionBlobs::new_unpruned(vec![]);

                let fluffy_complete_entry = BlockCompleteEntry {
                    block: block.clone(),
                    block_weight: 0,
                    txs: empty_txs,
                    pruned: false,
                };

                let new_fluffy_block = NewFluffyBlock {
                    b: fluffy_complete_entry,
                    current_blockchain_height: block_height + 1,
                };

                let mutex = self.peer_set.clone();

                async move {
                    let mut peer_set = mutex.lock().await;
                    let all_ready_peers = peer_set.all_ready();

                    let mut fut = FuturesOrdered::new();

                    for (_, svc) in all_ready_peers {
                        if svc.supports_fluffy_blocks() {
                            fut.push_back(svc.call(new_fluffy_block.clone().into()));
                        } else {
                            tracing::error!(
                                "Peer which doesn't support fluffy blocks is in the PeerSet"
                            )
                        }
                    }
                    peer_set.push_all_unready();

                    while let Some(_) = fut.next().await {}
                    Ok(BlockBroadCasterResponse::BlockBroadCasted)
                }
                .boxed()
            }
        }
    }
}

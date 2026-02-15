use std::{
    future::{ready, Future, Ready},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{stream::FuturesUnordered, StreamExt};
use indexmap::{IndexMap, IndexSet};
use rand::{seq::index::sample, thread_rng};
use tokio::sync::{mpsc::Receiver, Notify};
use tokio_util::sync::WaitForCancellationFutureOwned;
use tower::Service;

use cuprate_helper::cast::u64_to_usize;
use cuprate_p2p_core::{
    client::{Client, InternalPeerID},
    ConnectionDirection, NetworkZone,
};

mod client_wrappers;

pub use client_wrappers::ClientDropGuard;
use client_wrappers::StoredClient;

/// A request to the peer-set.
pub enum PeerSetRequest {
    /// The most claimed proof-of-work from a peer in the peer-set.
    MostPoWSeen,
    /// Peers with more cumulative difficulty than the given cumulative difficulty.
    ///
    /// Returned peers will be remembered and won't be returned from subsequent calls until the guard is dropped.
    PeersWithMorePoW(u128),
    /// A random outbound peer.
    ///
    /// The returned peer will be remembered and won't be returned from subsequent calls until the guard is dropped.
    StemPeer,
}

/// A response from the peer-set.
pub enum PeerSetResponse<N: NetworkZone> {
    /// [`PeerSetRequest::MostPoWSeen`]
    MostPoWSeen {
        /// The cumulative difficulty claimed.
        cumulative_difficulty: u128,
        /// The height claimed.
        height: usize,
        /// The claimed hash of the top block.
        top_hash: [u8; 32],
    },
    /// [`PeerSetRequest::PeersWithMorePoW`]
    ///
    /// Returned peers will be remembered and won't be returned from subsequent calls until the guard is dropped.
    PeersWithMorePoW(Vec<ClientDropGuard<N>>),
    /// [`PeerSetRequest::StemPeer`]
    ///
    /// The returned peer will be remembered and won't be returned from subsequent calls until the guard is dropped.
    StemPeer(Option<ClientDropGuard<N>>),
}

/// A [`Future`] that completes when a peer disconnects.
#[pin_project::pin_project]
struct ClosedConnectionFuture<N: NetworkZone> {
    #[pin]
    fut: WaitForCancellationFutureOwned,
    id: Option<InternalPeerID<N::Addr>>,
}

impl<N: NetworkZone> Future for ClosedConnectionFuture<N> {
    type Output = InternalPeerID<N::Addr>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        this.fut.poll(cx).map(|()| this.id.take().unwrap())
    }
}

/// A collection of all connected peers on a [`NetworkZone`].
pub(crate) struct PeerSet<N: NetworkZone> {
    /// The connected peers.
    peers: IndexMap<InternalPeerID<N::Addr>, StoredClient<N>>,
    /// A [`FuturesUnordered`] that resolves when a peer disconnects.
    closed_connections: FuturesUnordered<ClosedConnectionFuture<N>>,
    /// The [`InternalPeerID`]s of all outbound peers.
    outbound_peers: IndexSet<InternalPeerID<N::Addr>>,
    /// A channel of new peers from the inbound server or outbound connector.
    new_peers: Receiver<Client<N>>,
    /// The syncer wake handle.
    syncer_wake: Option<Arc<Notify>>,
}

impl<N: NetworkZone> PeerSet<N> {
    pub(crate) fn new(new_peers: Receiver<Client<N>>, syncer_wake: Option<Arc<Notify>>) -> Self {
        Self {
            peers: IndexMap::new(),
            closed_connections: FuturesUnordered::new(),
            outbound_peers: IndexSet::new(),
            new_peers,
            syncer_wake,
        }
    }

    /// Polls the new peers channel for newly connected peers.
    fn poll_new_peers(&mut self, cx: &mut Context<'_>) {
        while let Poll::Ready(Some(new_peer)) = self.new_peers.poll_recv(cx) {
            if new_peer.info.direction == ConnectionDirection::Outbound {
                self.outbound_peers.insert(new_peer.info.id);
            }

            self.closed_connections.push(ClosedConnectionFuture {
                fut: new_peer.info.handle.closed(),
                id: Some(new_peer.info.id),
            });

            self.peers
                .insert(new_peer.info.id, StoredClient::new(new_peer));

            // Wake the syncer to check if we are behind after adding a new peer.
            if let Some(syncer_wake) = &self.syncer_wake {
                syncer_wake.notify_one();
            }
        }
    }

    /// Remove disconnected peers from the peer set.
    fn remove_dead_peers(&mut self, cx: &mut Context<'_>) {
        while let Poll::Ready(Some(dead_peer)) = self.closed_connections.poll_next_unpin(cx) {
            let Some(peer) = self.peers.swap_remove(&dead_peer) else {
                continue;
            };

            if peer.client.info.direction == ConnectionDirection::Outbound {
                self.outbound_peers.swap_remove(&peer.client.info.id);
            }

            self.peers.swap_remove(&dead_peer);
        }
    }

    /// [`PeerSetRequest::MostPoWSeen`]
    fn most_pow_seen(&self) -> PeerSetResponse<N> {
        let most_pow_chain = self
            .peers
            .values()
            .map(|peer| {
                let core_sync_data = peer.client.info.core_sync_data.lock().unwrap();

                (
                    core_sync_data.cumulative_difficulty(),
                    u64_to_usize(core_sync_data.current_height),
                    core_sync_data.top_id,
                )
            })
            .max_by_key(|(cumulative_difficulty, ..)| *cumulative_difficulty)
            .unwrap_or_default();

        PeerSetResponse::MostPoWSeen {
            cumulative_difficulty: most_pow_chain.0,
            height: most_pow_chain.1,
            top_hash: most_pow_chain.2,
        }
    }

    /// [`PeerSetRequest::PeersWithMorePoW`]
    fn peers_with_more_pow(&self, cumulative_difficulty: u128) -> PeerSetResponse<N> {
        PeerSetResponse::PeersWithMorePoW(
            self.peers
                .values()
                .filter(|&client| {
                    !client.is_downloading_blocks()
                        && client
                            .client
                            .info
                            .core_sync_data
                            .lock()
                            .unwrap()
                            .cumulative_difficulty()
                            > cumulative_difficulty
                })
                .map(StoredClient::downloading_blocks_guard)
                .collect(),
        )
    }

    /// [`PeerSetRequest::StemPeer`]
    fn random_peer_for_stem(&self) -> PeerSetResponse<N> {
        PeerSetResponse::StemPeer(
            sample(
                &mut thread_rng(),
                self.outbound_peers.len(),
                self.outbound_peers.len(),
            )
            .into_iter()
            .find_map(|i| {
                let peer = self.outbound_peers.get_index(i).unwrap();
                let client = self.peers.get(peer).unwrap();
                (!client.is_a_stem_peer()).then(|| client.stem_peer_guard())
            }),
        )
    }
}

impl<N: NetworkZone> Service<PeerSetRequest> for PeerSet<N> {
    type Response = PeerSetResponse<N>;
    type Error = tower::BoxError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_new_peers(cx);
        self.remove_dead_peers(cx);

        // TODO: should we return `Pending` if we don't have any peers?

        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: PeerSetRequest) -> Self::Future {
        ready(match req {
            PeerSetRequest::MostPoWSeen => Ok(self.most_pow_seen()),
            PeerSetRequest::PeersWithMorePoW(cumulative_difficulty) => {
                Ok(self.peers_with_more_pow(cumulative_difficulty))
            }
            PeerSetRequest::StemPeer => Ok(self.random_peer_for_stem()),
        })
    }
}

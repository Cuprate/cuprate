use std::{
    future::{ready, Future, Ready},
    pin::Pin,
    task::{Context, Poll},
};

use futures::{stream::FuturesUnordered, StreamExt};
use indexmap::IndexMap;
use rand::{seq::IteratorRandom, thread_rng};
use tokio::sync::mpsc::Receiver;
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
    /// A channel of new peers from the inbound server or outbound connector.
    new_peers: Receiver<Client<N>>,
}

impl<N: NetworkZone> PeerSet<N> {
    pub(crate) fn new(new_peers: Receiver<Client<N>>) -> Self {
        Self {
            peers: IndexMap::new(),
            closed_connections: FuturesUnordered::new(),
            new_peers,
        }
    }

    /// Polls the new peers channel for newly connected peers.
    fn poll_new_peers(&mut self, cx: &mut Context<'_>) {
        while let Poll::Ready(Some(new_peer)) = self.new_peers.poll_recv(cx) {
            self.closed_connections.push(ClosedConnectionFuture {
                fut: new_peer.info.handle.closed(),
                id: Some(new_peer.info.id),
            });

            self.peers
                .insert(new_peer.info.id, StoredClient::new(new_peer));
        }
    }

    /// Remove disconnected peers from the peer set.
    fn remove_dead_peers(&mut self, cx: &mut Context<'_>) {
        while let Poll::Ready(Some(dead_peer)) = self.closed_connections.poll_next_unpin(cx) {
            let Some(peer) = self.peers.get(&dead_peer) else {
                continue;
            };

            // The id may have been reused by a new connection since this future was
            // created, so only remove the entry if it is the connection that closed.
            if !peer.client.info.handle.is_closed() {
                continue;
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
            self.peers
                .values()
                .filter(|client| {
                    client.client.info.direction == ConnectionDirection::Outbound
                        && !client.is_a_stem_peer()
                })
                .choose(&mut thread_rng())
                .map(StoredClient::stem_peer_guard),
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

#[cfg(test)]
mod tests {
    use std::{
        net::SocketAddr,
        sync::{Arc, Mutex},
    };

    use futures::FutureExt;
    use tokio::sync::mpsc;
    use tower::{Service, ServiceExt};

    use cuprate_p2p_core::{
        client::{mock_client, Client, InternalPeerID, PeerInformation},
        handles::{ConnectionHandle, HandleBuilder},
        ClearNet, ConnectionDirection, PeerRequest, PeerResponse, ProtocolResponse,
    };
    use cuprate_pruning::PruningSeed;
    use cuprate_wire::{common::PeerSupportFlags, BasicNodeData, CoreSyncData};

    use super::{PeerSet, PeerSetRequest, PeerSetResponse};

    fn mock_peer(
        id: InternalPeerID<SocketAddr>,
        direction: ConnectionDirection,
    ) -> (Client<ClearNet>, ConnectionHandle) {
        let (guard, handle) = HandleBuilder::new().build();

        let info = PeerInformation {
            id,
            basic_node_data: BasicNodeData {
                my_port: 0,
                network_id: [0; 16],
                peer_id: 0,
                support_flags: PeerSupportFlags::FLUFFY_BLOCKS,
                rpc_port: 0,
                rpc_credits_per_hash: 0,
            },
            handle: handle.clone(),
            direction,
            pruning_seed: PruningSeed::NotPruned,
            core_sync_data: Arc::new(Mutex::new(CoreSyncData {
                cumulative_difficulty: 1,
                cumulative_difficulty_top64: 0,
                current_height: 1,
                pruning_seed: 0,
                top_id: [0; 32],
                top_version: 0,
            })),
        };

        let svc = tower::service_fn(|_: PeerRequest| {
            async { Ok::<_, tower::BoxError>(PeerResponse::Protocol(ProtocolResponse::NA)) }.boxed()
        });

        (mock_client::<ClearNet, _>(info, guard, svc), handle)
    }

    #[tokio::test]
    async fn peer_id_reused_by_opposite_direction() {
        let id = InternalPeerID::KnownAddr("10.0.0.1:18080".parse::<SocketAddr>().unwrap());

        let (tx, rx) = mpsc::channel(4);
        let mut peer_set = PeerSet::<ClearNet>::new(rx);

        let (outbound, outbound_handle) = mock_peer(id, ConnectionDirection::Outbound);
        tx.send(outbound).await.unwrap();
        peer_set.ready().await.unwrap();
        assert_eq!(peer_set.peers.len(), 1);

        // Close it without polling the peer set, so the close future stays pending.
        outbound_handle.send_close_signal();
        while !outbound_handle.is_closed() {
            tokio::task::yield_now().await;
        }

        let (inbound, inbound_handle) = mock_peer(id, ConnectionDirection::Inbound);
        tx.send(inbound).await.unwrap();
        peer_set.ready().await.unwrap();

        // The stale close future must not evict the new connection.
        assert!(!inbound_handle.is_closed());
        assert_eq!(peer_set.peers.len(), 1);

        let PeerSetResponse::StemPeer(stem) =
            peer_set.call(PeerSetRequest::StemPeer).await.unwrap()
        else {
            panic!("peer set returned the wrong response");
        };
        assert!(stem.is_none());

        inbound_handle.send_close_signal();
        while !inbound_handle.is_closed() {
            tokio::task::yield_now().await;
        }
        peer_set.ready().await.unwrap();
        assert!(peer_set.peers.is_empty());
    }

    #[tokio::test]
    async fn outbound_peer_is_used_for_stem() {
        let id = InternalPeerID::KnownAddr("10.0.0.2:18080".parse::<SocketAddr>().unwrap());

        let (tx, rx) = mpsc::channel(4);
        let mut peer_set = PeerSet::<ClearNet>::new(rx);

        let (outbound, _handle) = mock_peer(id, ConnectionDirection::Outbound);
        tx.send(outbound).await.unwrap();
        peer_set.ready().await.unwrap();

        let PeerSetResponse::StemPeer(stem) =
            peer_set.call(PeerSetRequest::StemPeer).await.unwrap()
        else {
            panic!("peer set returned the wrong response");
        };
        assert!(stem.is_some());
    }
}

use std::{
    future::{ready, Ready},
    task::{Context, Poll},
};

use indexmap::{IndexMap, IndexSet};
use rand::{seq::index, thread_rng};
use tokio::sync::mpsc;
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
    PeersWithMorePoW(Vec<ClientDropGuard<N>>),
    /// [`PeerSetRequest::StemPeer`]
    StemPeer(Option<ClientDropGuard<N>>),
}

/// A collection of all connected peers on a [`NetworkZone`].
pub(crate) struct PeerSet<N: NetworkZone> {
    /// The connected peers.
    peers: IndexMap<InternalPeerID<N::Addr>, StoredClient<N>>,
    /// The [`InternalPeerID`]s of all outbound peers.
    outbound_peers: IndexSet<InternalPeerID<N::Addr>>,
    /// A channel of new peers from the inbound server or outbound connector.
    new_peers: mpsc::Receiver<Client<N>>,
}

impl<N: NetworkZone> PeerSet<N> {
    pub(crate) fn new(new_peers: mpsc::Receiver<Client<N>>) -> Self {
        Self {
            peers: IndexMap::new(),
            outbound_peers: IndexSet::new(),
            new_peers,
        }
    }

    /// Polls the new peers channel for newly connected peers.
    fn poll_new_peers(&mut self, cx: &mut Context<'_>) {
        while let Poll::Ready(Some(new_peer)) = self.new_peers.poll_recv(cx) {
            if new_peer.info.direction == ConnectionDirection::Outbound {
                self.outbound_peers.insert(new_peer.info.id);
            }

            self.peers
                .insert(new_peer.info.id, StoredClient::new(new_peer));
        }
    }

    /// Remove disconnected peers from the peer set.
    fn remove_dead_peers(&mut self) {
        let mut i = 0;
        while i < self.peers.len() {
            let peer = &self.peers[i];
            if peer.client.alive() {
                i += 1;
            } else {
                if peer.client.info.direction == ConnectionDirection::Outbound {
                    self.outbound_peers.swap_remove(&peer.client.info.id);
                }

                self.peers.swap_remove_index(i);
            }
        }
    }

    /// [`PeerSetRequest::MostPoWSeen`]
    fn most_pow_seen(&self) -> PeerSetResponse<N> {
        let mut most_pow_chain = (0, 0, [0; 32]);

        for peer in self.peers.values() {
            let core_sync_data = peer.client.info.core_sync_data.lock().unwrap();

            if core_sync_data.cumulative_difficulty() > most_pow_chain.0 {
                most_pow_chain = (
                    core_sync_data.cumulative_difficulty(),
                    u64_to_usize(core_sync_data.current_height),
                    core_sync_data.top_id,
                );
            }
        }

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
        let outbound_peers = index::sample(
            &mut thread_rng(),
            self.outbound_peers.len(),
            self.outbound_peers.len(),
        );

        for peer in outbound_peers
            .into_iter()
            .map(|i| self.outbound_peers.get_index(i).unwrap())
        {
            let client = self.peers.get(peer).unwrap();
            if client.is_a_stem_peer() {
                continue;
            }

            return PeerSetResponse::StemPeer(Some(client.stem_peer_guard()));
        }

        PeerSetResponse::StemPeer(None)
    }
}

impl<N: NetworkZone> Service<PeerSetRequest> for PeerSet<N> {
    type Response = PeerSetResponse<N>;
    type Error = tower::BoxError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_new_peers(cx);
        self.remove_dead_peers();

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

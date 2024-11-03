use cuprate_p2p_core::client::{Client, InternalPeerID};
use cuprate_p2p_core::NetworkZone;
use indexmap::IndexMap;
use std::future::{ready, Ready};
use std::task::{Context, Poll};
use tokio::sync::mpsc;
use tower::Service;

mod client_wrappers;

pub use client_wrappers::ClientDropGuard;
use client_wrappers::StoredClient;
use cuprate_helper::cast::u64_to_usize;

pub enum PeerSetRequest {
    MostPoWSeen,
    PeersWithMorePoW(u128),
}

pub enum PeerSetResponse<N: NetworkZone> {
    MostPoWSeen {
        cumulative_difficulty: u128,
        height: usize,
        top_hash: [u8; 32],
    },
    PeersWithMorePoW(Vec<ClientDropGuard<N>>),
}

pub(crate) struct PeerSet<N: NetworkZone> {
    peers: IndexMap<InternalPeerID<N::Addr>, StoredClient<N>>,
    new_peers: mpsc::Receiver<Client<N>>,
}

impl<N: NetworkZone> PeerSet<N> {
    pub(crate) fn new(new_peers: mpsc::Receiver<Client<N>>) -> Self {
        Self {
            peers: IndexMap::new(),
            new_peers,
        }
    }

    fn poll_new_peers(&mut self, cx: &mut Context<'_>) {
        while let Poll::Ready(Some(new_peer)) = self.new_peers.poll_recv(cx) {
            self.peers
                .insert(new_peer.info.id, StoredClient::new(new_peer));
        }
    }

    fn remove_dead_peers(&mut self) {
        let mut i = 0;
        while i < self.peers.len() {
            if self.peers[i].client.alive() {
                i += 1;
            } else {
                self.peers.swap_remove_index(i);
            }
        }
    }

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

    fn peers_with_more_pow(&self, cumulative_difficulty: u128) -> PeerSetResponse<N> {
        PeerSetResponse::PeersWithMorePoW(
            self.peers
                .values()
                .filter_map(|client| {
                    (!client.is_downloading_blocks()
                        && client
                            .client
                            .info
                            .core_sync_data
                            .lock()
                            .unwrap()
                            .cumulative_difficulty()
                            > cumulative_difficulty)
                        .then(|| client.downloading_blocks_guard())
                })
                .collect(),
        )
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
        })
    }
}

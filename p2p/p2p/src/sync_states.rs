//! # Sync States
//!
//! This module contains a [`PeerSyncSvc`], which keeps track of the claimed chain states of connected peers.
//! This allows checking if we are behind and getting a list of peers who claim they are ahead.
use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap, HashSet},
    future::{ready, Ready},
    task::{Context, Poll},
};

use futures::{stream::FuturesUnordered, StreamExt};
use tokio::sync::watch;
use tower::Service;

use cuprate_p2p_core::{
    client::InternalPeerID,
    handles::ConnectionHandle,
    services::{PeerSyncRequest, PeerSyncResponse},
    NetworkZone,
};
use cuprate_pruning::{PruningSeed, CRYPTONOTE_MAX_BLOCK_HEIGHT};
use cuprate_wire::CoreSyncData;

use crate::{client_pool::disconnect_monitor::PeerDisconnectFut, constants::SHORT_BAN};

/// The highest claimed sync info from our connected peers.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NewSyncInfo {
    /// The peers chain height.
    pub chain_height: u64,
    /// The peers top block's hash.
    pub top_hash: [u8; 32],
    /// The peers cumulative difficulty.
    pub cumulative_difficulty: u128,
}

/// A service that keeps track of our peers blockchains.
///
/// This is the service that handles:
/// 1. Finding out if we need to sync
/// 1. Giving the peers that should be synced _from_, to the requester
pub struct PeerSyncSvc<N: NetworkZone> {
    /// A map of cumulative difficulties to peers.
    cumulative_difficulties: BTreeMap<u128, HashSet<InternalPeerID<N::Addr>>>,
    /// A map of peers to cumulative difficulties.
    peers: HashMap<InternalPeerID<N::Addr>, (u128, PruningSeed)>,
    /// A watch channel for *a* top synced peer info.
    new_height_watcher: watch::Sender<NewSyncInfo>,
    /// The handle to the peer that has data in `new_height_watcher`.
    last_peer_in_watcher_handle: Option<ConnectionHandle>,
    /// A [`FuturesUnordered`] that resolves when a peer disconnects.
    closed_connections: FuturesUnordered<PeerDisconnectFut<N>>,
}

impl<N: NetworkZone> PeerSyncSvc<N> {
    /// Creates a new [`PeerSyncSvc`] with a [`Receiver`](watch::Receiver) that will be updated with
    /// the highest seen sync data, this makes no guarantees about which peer will be chosen in case of a tie.
    pub fn new() -> (Self, watch::Receiver<NewSyncInfo>) {
        let (watch_tx, mut watch_rx) = watch::channel(NewSyncInfo {
            chain_height: 0,
            top_hash: [0; 32],
            cumulative_difficulty: 0,
        });

        watch_rx.mark_unchanged();

        (
            Self {
                cumulative_difficulties: BTreeMap::new(),
                peers: HashMap::new(),
                new_height_watcher: watch_tx,
                last_peer_in_watcher_handle: None,
                closed_connections: FuturesUnordered::new(),
            },
            watch_rx,
        )
    }

    /// This function checks if any peers have disconnected, removing them if they have.
    fn poll_disconnected(&mut self, cx: &mut Context<'_>) {
        while let Poll::Ready(Some(peer_id)) = self.closed_connections.poll_next_unpin(cx) {
            tracing::trace!("Peer {peer_id} disconnected, removing from peers sync info service.");
            let (peer_cum_diff, _) = self.peers.remove(&peer_id).unwrap();

            let cum_diff_peers = self
                .cumulative_difficulties
                .get_mut(&peer_cum_diff)
                .unwrap();
            cum_diff_peers.remove(&peer_id);
            if cum_diff_peers.is_empty() {
                // If this was the last peer remove the whole entry for this cumulative difficulty.
                self.cumulative_difficulties.remove(&peer_cum_diff);
            }
        }
    }

    /// Returns a list of peers that claim to have a higher cumulative difficulty than `current_cum_diff`.
    fn peers_to_sync_from(
        &self,
        current_cum_diff: u128,
        block_needed: Option<usize>,
    ) -> Vec<InternalPeerID<N::Addr>> {
        self.cumulative_difficulties
            .range((current_cum_diff + 1)..)
            .flat_map(|(_, peers)| peers)
            .filter(|peer| {
                if let Some(block_needed) = block_needed {
                    // we just use CRYPTONOTE_MAX_BLOCK_HEIGHT as the blockchain height, this only means
                    // we don't take into account the tip blocks which are not pruned.
                    self.peers
                        .get(peer)
                        .unwrap()
                        .1
                        .has_full_block(block_needed, CRYPTONOTE_MAX_BLOCK_HEIGHT)
                } else {
                    true
                }
            })
            .copied()
            .collect()
    }

    /// Updates a peers sync state.
    fn update_peer_sync_info(
        &mut self,
        peer_id: InternalPeerID<N::Addr>,
        handle: ConnectionHandle,
        core_sync_data: CoreSyncData,
    ) -> Result<(), tower::BoxError> {
        tracing::trace!(
            "Received new core sync data from peer, top hash: {}",
            hex::encode(core_sync_data.top_id)
        );

        let new_cumulative_difficulty = core_sync_data.cumulative_difficulty();

        if let Some((old_cum_diff, _)) = self.peers.get_mut(&peer_id) {
            match (*old_cum_diff).cmp(&new_cumulative_difficulty) {
                Ordering::Equal => {
                    // If the cumulative difficulty of the peers chain hasn't changed then no need to update anything.
                    return Ok(());
                }
                Ordering::Greater => {
                    // This will only happen if a peer lowers its cumulative difficulty during the connection.
                    // This won't happen if a peer re-syncs their blockchain as then the connection would have closed.
                    tracing::debug!(
                        "Peer's claimed cumulative difficulty has dropped, closing connection and banning peer for: {} seconds.", SHORT_BAN.as_secs()
                    );
                    handle.ban_peer(SHORT_BAN);
                    return Err("Peers cumulative difficulty dropped".into());
                }
                Ordering::Less => (),
            }

            // Remove the old cumulative difficulty entry for this peer
            let old_cum_diff_peers = self.cumulative_difficulties.get_mut(old_cum_diff).unwrap();
            old_cum_diff_peers.remove(&peer_id);
            if old_cum_diff_peers.is_empty() {
                // If this was the last peer remove the whole entry for this cumulative difficulty.
                self.cumulative_difficulties.remove(old_cum_diff);
            }
            // update the cumulative difficulty
            *old_cum_diff = new_cumulative_difficulty;
        } else {
            // The peer is new so add it the list of peers.
            self.peers.insert(
                peer_id,
                (
                    new_cumulative_difficulty,
                    PruningSeed::decompress_p2p_rules(core_sync_data.pruning_seed)?,
                ),
            );

            // add it to the list of peers to watch for disconnection.
            self.closed_connections.push(PeerDisconnectFut {
                closed_fut: handle.closed(),
                peer_id: Some(peer_id),
            })
        }

        self.cumulative_difficulties
            .entry(new_cumulative_difficulty)
            .or_default()
            .insert(peer_id);

        // If the claimed cumulative difficulty is higher than the current one in the watcher
        // or if the peer in the watch has disconnected, update it.
        if self.new_height_watcher.borrow().cumulative_difficulty < new_cumulative_difficulty
            || self
                .last_peer_in_watcher_handle
                .as_ref()
                .is_some_and(|handle| handle.is_closed())
        {
            tracing::debug!(
                "Updating sync watcher channel with new highest seen cumulative difficulty: {new_cumulative_difficulty}"
            );
            let _ = self.new_height_watcher.send(NewSyncInfo {
                top_hash: core_sync_data.top_id,
                chain_height: core_sync_data.current_height,
                cumulative_difficulty: new_cumulative_difficulty,
            });
            self.last_peer_in_watcher_handle.replace(handle);
        }

        Ok(())
    }
}

impl<N: NetworkZone> Service<PeerSyncRequest<N>> for PeerSyncSvc<N> {
    type Response = PeerSyncResponse<N>;
    type Error = tower::BoxError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_disconnected(cx);

        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: PeerSyncRequest<N>) -> Self::Future {
        let res = match req {
            PeerSyncRequest::PeersToSyncFrom {
                current_cumulative_difficulty,
                block_needed,
            } => Ok(PeerSyncResponse::PeersToSyncFrom(self.peers_to_sync_from(
                current_cumulative_difficulty,
                block_needed,
            ))),
            PeerSyncRequest::IncomingCoreSyncData(peer_id, handle, sync_data) => self
                .update_peer_sync_info(peer_id, handle, sync_data)
                .map(|_| PeerSyncResponse::Ok),
        };

        ready(res)
    }
}

#[cfg(test)]
mod tests {
    use tower::{Service, ServiceExt};

    use cuprate_p2p_core::{
        client::InternalPeerID, handles::HandleBuilder, services::PeerSyncRequest,
    };
    use cuprate_wire::CoreSyncData;

    use cuprate_p2p_core::services::PeerSyncResponse;
    use cuprate_test_utils::test_netzone::TestNetZone;

    use super::PeerSyncSvc;

    #[tokio::test]
    async fn top_sync_channel_updates() {
        let (_g, handle) = HandleBuilder::new().build();

        let (mut svc, mut watch) = PeerSyncSvc::<TestNetZone<true, true, true>>::new();

        assert!(!watch.has_changed().unwrap());

        svc.ready()
            .await
            .unwrap()
            .call(PeerSyncRequest::IncomingCoreSyncData(
                InternalPeerID::Unknown(0),
                handle.clone(),
                CoreSyncData {
                    cumulative_difficulty: 1_000,
                    cumulative_difficulty_top64: 0,
                    current_height: 0,
                    pruning_seed: 0,
                    top_id: [0; 32],
                    top_version: 0,
                },
            ))
            .await
            .unwrap();

        assert!(watch.has_changed().unwrap());

        assert_eq!(watch.borrow().top_hash, [0; 32]);
        assert_eq!(watch.borrow().cumulative_difficulty, 1000);
        assert_eq!(watch.borrow_and_update().chain_height, 0);

        svc.ready()
            .await
            .unwrap()
            .call(PeerSyncRequest::IncomingCoreSyncData(
                InternalPeerID::Unknown(1),
                handle.clone(),
                CoreSyncData {
                    cumulative_difficulty: 1_000,
                    cumulative_difficulty_top64: 0,
                    current_height: 0,
                    pruning_seed: 0,
                    top_id: [0; 32],
                    top_version: 0,
                },
            ))
            .await
            .unwrap();

        assert!(!watch.has_changed().unwrap());

        svc.ready()
            .await
            .unwrap()
            .call(PeerSyncRequest::IncomingCoreSyncData(
                InternalPeerID::Unknown(2),
                handle.clone(),
                CoreSyncData {
                    cumulative_difficulty: 1_001,
                    cumulative_difficulty_top64: 0,
                    current_height: 0,
                    pruning_seed: 0,
                    top_id: [1; 32],
                    top_version: 0,
                },
            ))
            .await
            .unwrap();

        assert!(watch.has_changed().unwrap());

        assert_eq!(watch.borrow().top_hash, [1; 32]);
        assert_eq!(watch.borrow().cumulative_difficulty, 1001);
        assert_eq!(watch.borrow_and_update().chain_height, 0);
    }

    #[tokio::test]
    async fn peer_sync_info_updates() {
        let (_g, handle) = HandleBuilder::new().build();

        let (mut svc, _watch) = PeerSyncSvc::<TestNetZone<true, true, true>>::new();

        svc.ready()
            .await
            .unwrap()
            .call(PeerSyncRequest::IncomingCoreSyncData(
                InternalPeerID::Unknown(0),
                handle.clone(),
                CoreSyncData {
                    cumulative_difficulty: 1_000,
                    cumulative_difficulty_top64: 0,
                    current_height: 0,
                    pruning_seed: 0,
                    top_id: [0; 32],
                    top_version: 0,
                },
            ))
            .await
            .unwrap();

        assert_eq!(svc.peers.len(), 1);
        assert_eq!(svc.cumulative_difficulties.len(), 1);

        svc.ready()
            .await
            .unwrap()
            .call(PeerSyncRequest::IncomingCoreSyncData(
                InternalPeerID::Unknown(0),
                handle.clone(),
                CoreSyncData {
                    cumulative_difficulty: 1_001,
                    cumulative_difficulty_top64: 0,
                    current_height: 0,
                    pruning_seed: 0,
                    top_id: [0; 32],
                    top_version: 0,
                },
            ))
            .await
            .unwrap();

        assert_eq!(svc.peers.len(), 1);
        assert_eq!(svc.cumulative_difficulties.len(), 1);

        svc.ready()
            .await
            .unwrap()
            .call(PeerSyncRequest::IncomingCoreSyncData(
                InternalPeerID::Unknown(1),
                handle.clone(),
                CoreSyncData {
                    cumulative_difficulty: 10,
                    cumulative_difficulty_top64: 0,
                    current_height: 0,
                    pruning_seed: 0,
                    top_id: [0; 32],
                    top_version: 0,
                },
            ))
            .await
            .unwrap();

        assert_eq!(svc.peers.len(), 2);
        assert_eq!(svc.cumulative_difficulties.len(), 2);

        let PeerSyncResponse::PeersToSyncFrom(peers) = svc
            .ready()
            .await
            .unwrap()
            .call(PeerSyncRequest::PeersToSyncFrom {
                block_needed: None,
                current_cumulative_difficulty: 0,
            })
            .await
            .unwrap()
        else {
            panic!("Wrong response for request.")
        };

        assert!(
            peers.contains(&InternalPeerID::Unknown(0))
                && peers.contains(&InternalPeerID::Unknown(1))
        )
    }
}

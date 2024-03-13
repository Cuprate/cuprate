use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap, HashSet},
    future::{ready, Future, Ready},
    pin::Pin,
    task::{Context, Poll},
};

use futures::{stream::FuturesUnordered, StreamExt};
use tokio::sync::watch;
use tokio_util::sync::WaitForCancellationFutureOwned;
use tower::Service;

use monero_p2p::{
    client::InternalPeerID,
    handles::ConnectionHandle,
    services::{PeerSyncRequest, PeerSyncResponse},
    NetworkZone,
};
use monero_wire::CoreSyncData;

use crate::constants::SHORT_BAN;

pub struct NewSyncInfo {
    chain_height: u64,
    top_hash: [u8; 32],
    cumulative_difficulty: u128,
}

/// A service that keeps track of our peers blockchains.
///
/// This is the service that handles finding out if we need to sync and giving the peers that should
/// be synced from to the requester.
pub struct PeerSyncSvc<N: NetworkZone> {
    /// A map of cumulative difficulties to peers.
    cumulative_difficulties: BTreeMap<u128, HashSet<InternalPeerID<N::Addr>>>,
    /// A map of peers to cumulative difficulties.
    peers: HashMap<InternalPeerID<N::Addr>, u128>,
    /// A watch channel for *a* top synced peer info.
    ///
    /// This is guaranteed to hold the sync info of a peer with the highest cumulative difficulty seen,
    /// this makes no guarantees about which peer will be chosen in case of a tie.
    new_height_watcher: watch::Sender<NewSyncInfo>,
    /// The handle to the peer that has data in `new_height_watcher`.
    last_peer_in_watcher_handle: ConnectionHandle,
    /// A [`FuturesUnordered`] that resolves when a peer disconnects.
    closed_connections: FuturesUnordered<PeerDisconnectFut<N>>,
}

impl<N: NetworkZone> PeerSyncSvc<N> {
    fn poll_disconnected(&mut self, cx: &mut Context<'_>) {
        while let Poll::Ready(Some(peer_id)) = self.closed_connections.poll_next_unpin(cx) {
            tracing::trace!("Peer {peer_id} disconnected, removing from peers sync info service.");
            let peer_cum_diff = self.peers.remove(&peer_id).unwrap();

            let cum_dif_peers = self
                .cumulative_difficulties
                .get_mut(&peer_cum_diff)
                .unwrap();
            cum_dif_peers.remove(&peer_id);
            if cum_dif_peers.is_empty() {
                // If this was the last peer remove the whole entry for this cumulative difficulty.
                self.cumulative_difficulties.remove(&peer_cum_diff);
            }
        }
    }

    /// Returns a list of peers that claim to have a higher cumulative difficulty than `current_cum_dif`.
    fn peers_to_sync_from(&self, current_cum_dif: u128) -> Vec<InternalPeerID<N::Addr>> {
        self.cumulative_difficulties
            .range((current_cum_dif + 1)..)
            .flat_map(|(_, peers)| peers)
            .copied()
            .collect()
    }

    /// Updates a peers sync state.
    fn update_peer_sync_info(
        &mut self,
        peer_id: InternalPeerID<N::Addr>,
        mut handle: ConnectionHandle,
        core_sync_data: CoreSyncData,
    ) -> Result<(), tower::BoxError> {
        tracing::trace!(
            "Received new core sync data from peer, top hash: {}",
            hex::encode(core_sync_data.top_id)
        );

        let new_cumulative_difficulty = core_sync_data.cumulative_difficulty();

        if let Some(old_cum_dif) = self.peers.insert(peer_id, new_cumulative_difficulty) {
            match old_cum_dif.cmp(&new_cumulative_difficulty) {
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
                }
                Ordering::Less => (),
            }

            // Remove the old cumulative difficulty entry for this peer
            let old_cum_dif_peers = self.cumulative_difficulties.get_mut(&old_cum_dif).unwrap();
            old_cum_dif_peers.remove(&peer_id);
            if old_cum_dif_peers.is_empty() {
                // If this was the last peer remove the whole entry for this cumulative difficulty.
                self.cumulative_difficulties.remove(&old_cum_dif);
            }
        } else {
            // The peer is new so add it to the list of peers to watch for disconnection.
            self.closed_connections.push(PeerDisconnectFut {
                closed_fut: handle.closed(),
                peer_id: Some(peer_id),
            })
        }

        self.cumulative_difficulties
            .entry(new_cumulative_difficulty)
            .or_default()
            .insert(peer_id);

        if self.new_height_watcher.borrow().cumulative_difficulty < new_cumulative_difficulty
            || self.last_peer_in_watcher_handle.is_closed()
        {
            tracing::debug!(
                "Updating sync watcher channel with new highest seen cumulative difficulty."
            );
            let _ = self.new_height_watcher.send(NewSyncInfo {
                top_hash: core_sync_data.top_id,
                chain_height: core_sync_data.current_height,
                cumulative_difficulty: new_cumulative_difficulty,
            });
            self.last_peer_in_watcher_handle = handle;
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
            PeerSyncRequest::PeersToSyncFrom(current_cum_dif) => Ok(
                PeerSyncResponse::PeersToSyncFrom(self.peers_to_sync_from(current_cum_dif)),
            ),
            PeerSyncRequest::IncomingCoreSyncData(peer_id, handle, sync_data) => self
                .update_peer_sync_info(peer_id, handle, sync_data)
                .map(|_| PeerSyncResponse::Ok),
        };

        ready(res)
    }
}

#[pin_project::pin_project]
struct PeerDisconnectFut<N: NetworkZone> {
    #[pin]
    closed_fut: WaitForCancellationFutureOwned,
    peer_id: Option<InternalPeerID<N::Addr>>,
}

impl<N: NetworkZone> Future for PeerDisconnectFut<N> {
    type Output = InternalPeerID<N::Addr>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        this.closed_fut
            .poll(cx)
            .map(|_| this.peer_id.take().unwrap())
    }
}

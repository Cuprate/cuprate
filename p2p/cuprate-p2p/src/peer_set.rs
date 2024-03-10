//! This module contains the peer set and related functionality.
//!
use std::{
    cmp::Ordering,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{
    lock::{Mutex as AsyncMutex, MutexGuard, OwnedMutexGuard, OwnedMutexLockFuture},
    ready, FutureExt,
};
use indexmap::{IndexMap, IndexSet};
use rand::prelude::*;
use tokio::sync::mpsc;
use tower::{load::Load, ServiceExt};
use tracing::instrument;

use monero_p2p::{
    client::{Client, InternalPeerID, PeakEwmaClient},
    ConnectionDirection, NetworkZone,
};

use crate::connection_maintainer::MakeConnectionRequest;

/// A locked peer set that can be shared to different tasks.
pub struct LockedPeerSet<N: NetworkZone> {
    /// The peer set wrapped in an arc mutex.
    set: Arc<AsyncMutex<InnerPeerSet<N>>>,
    /// The state of an attempt to acquire the lock on the peer set.
    state: PeerSetLockState<N>,
}

impl<N: NetworkZone> Clone for LockedPeerSet<N> {
    fn clone(&self) -> Self {
        Self {
            set: self.set.clone(),
            state: PeerSetLockState::Locked,
        }
    }
}

impl<N: NetworkZone> LockedPeerSet<N> {
    pub async fn acquire(&mut self) -> MutexGuard<'_, InnerPeerSet<N>> {
        self.set.lock().await
    }

    /// Acquires the [`InnerPeerSet`] using a poll interface.
    pub fn poll_acquire(&mut self, cx: &mut Context<'_>) -> Poll<OwnedMutexGuard<InnerPeerSet<N>>> {
        loop {
            match &mut self.state {
                PeerSetLockState::Locked => {
                    self.state = PeerSetLockState::Pending(self.set.clone().lock_owned());
                }
                PeerSetLockState::Pending(lock_fut) => {
                    let guard = ready!(lock_fut.poll_unpin(cx));
                    self.state = PeerSetLockState::Locked;
                    return Poll::Ready(guard);
                }
            }
        }
    }
}

/// The state of the peer set lock
enum PeerSetLockState<N: NetworkZone> {
    /// Locked
    Locked,
    /// Waiting our turn to access the peer set.
    Pending(OwnedMutexLockFuture<InnerPeerSet<N>>),
}

/// The peer-set.
///
/// This struct holds peers currently connected on a certain [`NetworkZone`].
/// The peer-set is what the routing methods use to get peers
pub struct InnerPeerSet<N: NetworkZone> {
    new_peers_rx: mpsc::Receiver<Client<N>>,
    /// A channel to the outbound connection maker to make a new connection
    make_new_connections_tx: mpsc::Sender<MakeConnectionRequest>,

    outbound_peers: IndexSet<InternalPeerID<N::Addr>>,
    inbound_peers: IndexSet<InternalPeerID<N::Addr>>,

    ready_peers: IndexMap<InternalPeerID<N::Addr>, PeakEwmaClient<N>>,

    pending_peers: IndexMap<InternalPeerID<N::Addr>, PeakEwmaClient<N>>,
}

impl<N: NetworkZone> InnerPeerSet<N> {
    fn remove_peer(&mut self, id: &InternalPeerID<N::Addr>) {
        tracing::debug!("Removing Peer: {} from PeerSet", id);

        self.outbound_peers.swap_remove(id);
        self.inbound_peers.swap_remove(id);

        self.ready_peers.swap_remove(id);
        self.pending_peers.swap_remove(id);
    }

    #[instrument(level = "debug", skip(self))]
    fn check_pending_peers(&mut self) {
        let mut new_ready_peers = Vec::new();
        let mut failed_peers = Vec::new();

        for (peer_id, peer) in self.pending_peers.iter_mut() {
            match peer.ready().now_or_never() {
                Some(Ok(_)) => new_ready_peers.push(*peer_id),
                Some(Err(e)) => {
                    tracing::debug!("Peer: {}'s client gave an Err: {}", peer_id, e);
                    failed_peers.push(*peer_id);
                }
                None => (),
            }
        }

        for failed_peer_id in failed_peers {
            self.remove_peer(&failed_peer_id)
        }

        for peer_id in new_ready_peers {
            tracing::debug!("Peer: {} is now ready, adding it to ready list.", peer_id);

            let peer = self.pending_peers.swap_remove(&peer_id).unwrap();
            let already_in_ready_list = self.ready_peers.insert(peer_id, peer).is_some();

            assert!(!already_in_ready_list);
        }
    }

    /// Returns a peer address selected from a load balancing algorithm.
    pub fn load_balanced_peer(&mut self) -> Option<InternalPeerID<N::Addr>> {
        let is_ok = |client: &mut PeakEwmaClient<N>| {
            let Some(res) = client.ready().now_or_never() else {
                // TODO: This should not happen but for now make this a warning just in case it does.
                tracing::warn!("Peer was not ready in peer set when it should be.");
                return false;
            };

            res.is_ok()
        };

        loop {
            match self.ready_peers.len() {
                0 => return None,
                1 => {
                    let (&addr, client) = self.ready_peers.get_index_mut(0).unwrap();

                    if !is_ok(client) {
                        tracing::debug!("Peer {} had an error or was not ready.", addr);
                        self.remove_peer(&addr);
                        continue;
                    }

                    return Some(addr);
                }
                _ => {
                    let indexs =
                        rand::seq::index::sample(&mut thread_rng(), self.ready_peers.len(), 2);

                    // Get the first random peer.
                    let (&peer_1_addr, peer_1_client) =
                        self.ready_peers.get_index_mut(indexs.index(0)).unwrap();
                    // Check the peer is ready and has not had an error.
                    if !is_ok(peer_1_client) {
                        tracing::debug!("Peer {} had an error or was not ready.", peer_1_addr);
                        self.remove_peer(&peer_1_addr);
                        continue;
                    }
                    // Get peer1's load.
                    let peer_1_load = peer_1_client.load();

                    // Get the second random peer.
                    let (&peer_2_addr, peer_2_client) =
                        self.ready_peers.get_index_mut(indexs.index(1)).unwrap();
                    // Check the peer is ready and has not had an error.
                    if !is_ok(peer_2_client) {
                        tracing::debug!("Peer {} had an error or was not ready.", peer_2_addr);
                        self.remove_peer(&peer_2_addr);
                        continue;
                    }
                    // Get peer2's load.
                    let peer_2_load = peer_2_client.load();

                    tracing::trace!(
                        "Selecting less loaded peer between p1: {} l: {:?}, p2: {}, l: {:?}",
                        peer_1_addr,
                        peer_1_load,
                        peer_2_addr,
                        peer_2_load
                    );

                    let peer = match peer_1_load.partial_cmp(&peer_2_load) {
                        None | Some(Ordering::Less) | Some(Ordering::Equal) => peer_1_addr,
                        Some(Ordering::Greater) => peer_2_addr,
                    };

                    tracing::debug!("Selected P2C peer: {}", peer);
                    return Some(peer);
                }
            }
        }
    }

    pub fn get_peer(&mut self, addr: InternalPeerID<N::Addr>) -> Option<&mut PeakEwmaClient<N>> {
        self.ready_peers.get_mut(&addr)
    }

    pub fn take_peer(&mut self, addr: &InternalPeerID<N::Addr>) -> Option<PeakEwmaClient<N>> {
        let peer = self.ready_peers.swap_remove(addr)?;

        match peer.info.direction {
            ConnectionDirection::InBound => self.inbound_peers.swap_remove(addr),
            ConnectionDirection::OutBound => self.outbound_peers.swap_remove(addr),
        };

        Some(peer)
    }

    pub fn number_ready(&self) -> usize {
        self.ready_peers.len()
    }
}

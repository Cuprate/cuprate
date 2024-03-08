//! This module contains the peer set and related functionality.
//!
use std::{
    sync::Arc,
    task::{Context, Poll},
};

use futures::{
    lock::{Mutex as AsyncMutex, OwnedMutexGuard, OwnedMutexLockFuture},
    ready, FutureExt,
};
use indexmap::{IndexMap, IndexSet};
use tokio::sync::mpsc;
use tower::ServiceExt;
use tracing::instrument;

use monero_p2p::{
    client::{Client, InternalPeerID},
    NetworkZone,
};

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

    ready_peers: IndexMap<InternalPeerID<N::Addr>, Client<N>>,

    pending_peers: IndexMap<InternalPeerID<N::Addr>, Client<N>>,
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
}

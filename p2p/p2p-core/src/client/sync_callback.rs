use std::{
    fmt::{Debug, Formatter},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use tokio::sync::{watch, Notify};

/// A callback for the syncer, called with a peer's cumulative difficulty when sync data is updated.
#[derive(Clone)]
pub struct PeerSyncCallback(Arc<PeerSyncCallbackInner>);

struct PeerSyncCallbackInner {
    /// Returns `true` if the syncer should be woken for this peer's cumulative difficulty.
    filter: Box<dyn Fn(u128) -> bool + Send + Sync>,
    /// The syncer wake handle.
    wake: Arc<Notify>,
    /// Whether we have at least one connected peer.
    has_peers: AtomicBool,
    /// Tracks how many incoming blocks are currently being processed via fluffy relay.
    incoming_block_tx: watch::Sender<u32>,
}

impl PeerSyncCallback {
    /// Create a new [`PeerSyncCallback`].
    pub fn new(filter: Box<dyn Fn(u128) -> bool + Send + Sync>, wake: Arc<Notify>) -> Self {
        Self(Arc::new(PeerSyncCallbackInner {
            filter,
            wake,
            has_peers: AtomicBool::new(false),
            incoming_block_tx: watch::Sender::new(0),
        }))
    }

    /// Wake the syncer if the peer's cumulative difficulty passes the filter.
    pub fn call(&self, peer_cd: u128) {
        if (self.0.filter)(peer_cd) {
            self.0.wake.notify_one();
        }
    }

    /// Wake the syncer unconditionally, bypassing the filter.
    pub fn wake_unconditionally(&self) {
        self.0.wake.notify_one();
    }

    /// Wake the syncer when we get our first peers, marking that peers are now present.
    ///
    /// Returns `true` if this was the first peer and the syncer was woken.
    pub fn wake_on_first_peers(&self) -> bool {
        if !self.0.has_peers.swap(true, Ordering::Relaxed) {
            self.0.wake.notify_one();
            return true;
        }
        false
    }

    /// Reset the peer tracking, allowing [`wake_on_first_peers`](Self::wake_on_first_peers) to fire again.
    pub fn wake_on_first_peers_arm(&self) {
        self.0.has_peers.store(false, Ordering::Relaxed);
    }

    /// Wait for the syncer to be notified.
    pub async fn notified(&self) {
        self.0.wake.notified().await;
    }

    /// Mark that an incoming fluffy block is being processed.
    pub fn incoming_block_in_flight(&self) {
        self.0.incoming_block_tx.send_modify(|c| *c += 1);
    }

    /// Mark that an incoming fluffy block has finished processing.
    pub fn incoming_block_done(&self) {
        self.0.incoming_block_tx.send_modify(|c| *c -= 1);
    }

    /// Subscribe to incoming block processing state changes.
    pub fn subscribe_incoming_block(&self) -> watch::Receiver<u32> {
        self.0.incoming_block_tx.subscribe()
    }
}

impl Debug for PeerSyncCallback {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("PeerSyncCallback")
    }
}

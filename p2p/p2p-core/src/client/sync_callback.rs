use std::{
    fmt::{Debug, Formatter},
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    },
};

use tokio::sync::{watch, Notify};

/// The reason the syncer was woken.
#[derive(Debug, Clone, Copy)]
pub enum WakeReason {
    Recheck = 1,
    BehindPeers = 2,
}

impl WakeReason {
    const fn from_raw(value: u8) -> Self {
        match value {
            1 => Self::Recheck,
            2 => Self::BehindPeers,
            _ => unreachable!(),
        }
    }
}

/// A callback for the syncer, called with a peer's cumulative difficulty when sync data is updated.
#[derive(Clone)]
pub struct PeerSyncCallback(Arc<PeerSyncCallbackInner>);

struct PeerSyncCallbackInner {
    /// Returns `true` if the syncer should be woken for this peer's cumulative difficulty.
    filter: Box<dyn Fn(u128) -> bool + Send + Sync>,
    /// The syncer wake handle.
    wake: Notify,
    /// The reason for the most recent wake, using priority-based coalescing via `fetch_max`.
    wake_reason: AtomicU8,
    /// Whether we have at least one connected peer.
    has_peers: AtomicBool,
    /// Tracks how many incoming blocks are currently being processed via fluffy relay.
    incoming_block_tx: watch::Sender<u32>,
}

impl PeerSyncCallback {
    /// Create a new [`PeerSyncCallback`].
    pub fn new(filter: Box<dyn Fn(u128) -> bool + Send + Sync>) -> Self {
        Self(Arc::new(PeerSyncCallbackInner {
            filter,
            wake: Notify::new(),
            wake_reason: AtomicU8::new(0),
            has_peers: AtomicBool::new(false),
            incoming_block_tx: watch::Sender::new(0),
        }))
    }

    /// Wake the syncer if the peer's cumulative difficulty passes the filter.
    pub fn call(&self, peer_cd: u128) {
        if (self.0.filter)(peer_cd) {
            self.0
                .wake_reason
                .fetch_max(WakeReason::BehindPeers as u8, Ordering::Relaxed);
            self.0.wake.notify_one();
        }
    }

    /// Force wake the syncer without the filter.
    pub fn wake_unconditionally(&self) {
        self.0
            .wake_reason
            .fetch_max(WakeReason::Recheck as u8, Ordering::Relaxed);
        self.0.wake.notify_one();
    }

    /// Wake the syncer when we get our first peers, marking that peers are now present.
    ///
    /// Returns `true` if this was the first peer and the syncer was woken.
    pub fn wake_on_first_peers(&self) -> bool {
        if !self.0.has_peers.swap(true, Ordering::Relaxed) {
            self.0
                .wake_reason
                .fetch_max(WakeReason::Recheck as u8, Ordering::Relaxed);
            self.0.wake.notify_one();
            return true;
        }
        false
    }

    /// Reset for if we lose all peers, allowing [`wake_on_first_peers`](Self::wake_on_first_peers) to fire again.
    pub fn wake_on_first_peers_arm(&self) {
        self.0.has_peers.store(false, Ordering::Relaxed);
    }

    /// Clear any pending [`BehindPeers`](WakeReason::BehindPeers) reason.
    pub fn clear_pending_behind_peers(&self) {
        #[expect(clippy::let_underscore_must_use)]
        let _ = self.0.wake_reason.compare_exchange_weak(
            WakeReason::BehindPeers as u8,
            0,
            Ordering::Relaxed,
            Ordering::Relaxed,
        );
    }

    /// Wait for the syncer to be notified, returning the reason for the wake.
    pub async fn notified(&self) -> WakeReason {
        loop {
            let notified = self.0.wake.notified();
            match self.0.wake_reason.swap(0, Ordering::Relaxed) {
                0 => notified.await,
                r => return WakeReason::from_raw(r),
            }
        }
    }

    /// Mark that an incoming fluffy block is being processed.
    pub fn incoming_block_guard(&self) -> IncomingBlockGuard {
        self.0.incoming_block_tx.send_modify(|c| {
            *c = c.checked_add(1).expect("incoming block count overflow");
        });
        IncomingBlockGuard(self.clone())
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

/// A guard that increments the incoming block count on creation and decrements it on drop.
pub struct IncomingBlockGuard(PeerSyncCallback);

impl Drop for IncomingBlockGuard {
    fn drop(&mut self) {
        self.0 .0.incoming_block_tx.send_modify(|c| {
            *c = c.checked_sub(1).expect("incoming block count underflow");
        });
    }
}

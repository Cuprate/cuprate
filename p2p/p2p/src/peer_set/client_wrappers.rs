use std::{
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use cuprate_p2p_core::{
    client::{Client, WeakClient},
    NetworkZone,
};

/// A client stored in the peer-set.
pub(super) struct StoredClient<N: NetworkZone> {
    pub client: Client<N>,
    /// An [`AtomicBool`] for if the peer is currently downloading blocks.
    downloading_blocks: Arc<AtomicBool>,
    /// An [`AtomicBool`] for if the peer is currently being used to stem txs.
    stem_peer: Arc<AtomicBool>,
}

impl<N: NetworkZone> StoredClient<N> {
    pub(super) fn new(client: Client<N>) -> Self {
        Self {
            client,
            downloading_blocks: Arc::new(AtomicBool::new(false)),
            stem_peer: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Returns [`true`] if the [`StoredClient`] is currently downloading blocks.
    pub(super) fn is_downloading_blocks(&self) -> bool {
        self.downloading_blocks.load(Ordering::Relaxed)
    }

    /// Returns [`true`] if the [`StoredClient`] is currently being used to stem txs.
    pub(super) fn is_a_stem_peer(&self) -> bool {
        self.stem_peer.load(Ordering::Relaxed)
    }

    /// Returns a [`ClientDropGuard`] that while it is alive keeps the [`StoredClient`] in the downloading blocks state.
    pub(super) fn downloading_blocks_guard(&self) -> ClientDropGuard<N> {
        self.downloading_blocks.store(true, Ordering::Relaxed);

        ClientDropGuard {
            client: self.client.downgrade(),
            bool: Arc::clone(&self.downloading_blocks),
        }
    }

    /// Returns a [`ClientDropGuard`] that while it is alive keeps the [`StoredClient`] in the stemming peers state.
    pub(super) fn stem_peer_guard(&self) -> ClientDropGuard<N> {
        self.stem_peer.store(true, Ordering::Relaxed);

        ClientDropGuard {
            client: self.client.downgrade(),
            bool: Arc::clone(&self.stem_peer),
        }
    }
}

/// A [`Drop`] guard for a client returned from the peer-set.
pub struct ClientDropGuard<N: NetworkZone> {
    client: WeakClient<N>,
    bool: Arc<AtomicBool>,
}

impl<N: NetworkZone> Deref for ClientDropGuard<N> {
    type Target = WeakClient<N>;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl<N: NetworkZone> DerefMut for ClientDropGuard<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

impl<N: NetworkZone> Drop for ClientDropGuard<N> {
    fn drop(&mut self) {
        self.bool.store(false, Ordering::Relaxed);
    }
}

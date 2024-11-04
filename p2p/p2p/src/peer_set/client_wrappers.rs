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

pub(super) struct StoredClient<N: NetworkZone> {
    pub client: Client<N>,
    downloading_blocks: Arc<AtomicBool>,
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

    pub(super) fn is_downloading_blocks(&self) -> bool {
        self.downloading_blocks.load(Ordering::Relaxed)
    }

    pub(super) fn is_a_stem_peer(&self) -> bool {
        self.stem_peer.load(Ordering::Relaxed)
    }

    pub(super) fn downloading_blocks_guard(&self) -> ClientDropGuard<N> {
        ClientDropGuard {
            client: self.client.downgrade(),
            bool: Arc::clone(&self.downloading_blocks),
        }
    }

    pub(super) fn stem_peer_guard(&self) -> ClientDropGuard<N> {
        ClientDropGuard {
            client: self.client.downgrade(),
            bool: Arc::clone(&self.stem_peer),
        }
    }
}

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

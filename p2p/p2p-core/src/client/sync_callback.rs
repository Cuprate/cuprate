use std::{
    fmt::{Debug, Formatter},
    sync::Arc,
};

use cuprate_wire::CoreSyncData;

/// Callbacks invoked when a peer's sync data changes or the peer disconnects.
#[derive(Clone)]
pub struct PeerSyncCallback {
    on_sync: Arc<dyn Fn(&CoreSyncData) + Send + Sync>,
    on_disconnect: Arc<dyn Fn() + Send + Sync>,
}

impl PeerSyncCallback {
    /// Create a new [`PeerSyncCallback`].
    pub fn new(
        on_sync: impl Fn(&CoreSyncData) + Send + Sync + 'static,
        on_disconnect: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        Self {
            on_sync: Arc::new(on_sync),
            on_disconnect: Arc::new(on_disconnect),
        }
    }

    /// Call the callback with the peer's [`CoreSyncData`].
    pub fn call(&self, data: &CoreSyncData) {
        (self.on_sync)(data);
    }

    /// Notify the callback that the peer disconnected.
    pub(crate) fn disconnected(&self) {
        (self.on_disconnect)();
    }
}

impl Debug for PeerSyncCallback {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("PeerSyncCallback")
    }
}

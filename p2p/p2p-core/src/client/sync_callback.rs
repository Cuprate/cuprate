use std::{
    fmt::{Debug, Formatter},
    sync::Arc,
};

use cuprate_wire::CoreSyncData;

/// A callback invoked with a peer's [`CoreSyncData`].
#[derive(Clone)]
pub struct PeerSyncCallback(Arc<dyn Fn(&CoreSyncData) + Send + Sync>);

impl PeerSyncCallback {
    /// Create a new [`PeerSyncCallback`].
    pub fn new(f: impl Fn(&CoreSyncData) + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }

    /// Call the callback with the peer's [`CoreSyncData`].
    pub fn call(&self, data: &CoreSyncData) {
        (self.0)(data);
    }
}

impl Debug for PeerSyncCallback {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("PeerSyncCallback")
    }
}

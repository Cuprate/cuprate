//!
use std::{sync::OnceLock, time::Duration};

use futures::SinkExt;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;

#[derive(Default, Debug)]
pub struct HandleBuilder {
    permit: Option<OwnedSemaphorePermit>,
}

impl HandleBuilder {
    pub fn new() -> Self {
        Self { permit: None }
    }

    pub fn with_permit(mut self, permit: OwnedSemaphorePermit) -> Self {
        self.permit = Some(permit);
        self
    }

    pub fn build(self) -> (ConnectionGuard, ConnectionHandle) {
        let token = CancellationToken::new();

        (
            ConnectionGuard {
                token: token.clone(),
                permit: self.permit.expect("connection permit was not set!"),
            },
            ConnectionHandle {
                token: token.clone(),
                ban: OnceLock::new(),
            },
        )
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BanPeer(pub Duration);

/// A struct given to the connection task.
pub struct ConnectionGuard {
    token: CancellationToken,
    permit: OwnedSemaphorePermit,
}

impl ConnectionGuard {
    pub fn should_shutdown(&self) -> bool {
        self.token.is_cancelled()
    }
    pub fn connection_closed(&self) {
        self.token.cancel()
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.token.cancel()
    }
}

/// A handle given to a task that needs to close this connection and find out if the connection has
/// been banned.
pub struct ConnectionHandle {
    token: CancellationToken,
    ban: OnceLock<BanPeer>,
}

impl ConnectionHandle {
    pub fn ban_peer(&self, duration: Duration) {
        let _ = self.ban.set(BanPeer(duration));
    }
    pub fn is_closed(&self) -> bool {
        self.token.is_cancelled()
    }
    pub fn check_should_ban(&mut self) -> Option<BanPeer> {
        self.ban.get().copied()
    }
    pub fn send_close_signal(&self) {
        self.token.cancel()
    }
}

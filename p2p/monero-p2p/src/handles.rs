//!
use std::time::Duration;

use futures::{channel::mpsc, SinkExt};
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

    pub fn build(self) -> (ConnectionGuard, ConnectionHandle, PeerHandle) {
        let token = CancellationToken::new();
        let (tx, rx) = mpsc::channel(0);

        (
            ConnectionGuard {
                token: token.clone(),
                permit: self.permit.expect("connection permit was not set!"),
            },
            ConnectionHandle {
                token: token.clone(),
                ban: rx,
            },
            PeerHandle { ban: tx, token },
        )
    }
}

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
    ban: mpsc::Receiver<BanPeer>,
}

impl ConnectionHandle {
    pub fn is_closed(&self) -> bool {
        self.token.is_cancelled()
    }
    pub fn check_should_ban(&mut self) -> Option<BanPeer> {
        self.ban.try_next().unwrap_or(None)
    }
    pub fn send_close_signal(&self) {
        self.token.cancel()
    }
}

/// A handle given to a task that needs to be able to ban a peer.
#[derive(Clone)]
pub struct PeerHandle {
    token: CancellationToken,
    ban: mpsc::Sender<BanPeer>,
}

impl PeerHandle {
    pub fn ban_peer(&mut self, duration: Duration) {
        // This channel won't be dropped and if it's full the peer has already been banned.
        let _ = self.ban.try_send(BanPeer(duration));
        self.token.cancel()
    }
}

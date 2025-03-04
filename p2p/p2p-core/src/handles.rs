//! Connection Handles.
//!
//! This module contains the [`ConnectionHandle`] which allows banning a peer, disconnecting a peer and
//! checking if the peer is still connected.
use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use tokio::sync::OwnedSemaphorePermit;
use tokio_util::sync::{CancellationToken, WaitForCancellationFutureOwned};

/// A [`ConnectionHandle`] builder.
#[derive(Default, Debug)]
pub struct HandleBuilder {
    permit: Option<OwnedSemaphorePermit>,
}

impl HandleBuilder {
    /// Create a new builder.
    pub const fn new() -> Self {
        Self { permit: None }
    }

    /// Sets the permit for this connection.
    #[must_use]
    pub fn with_permit(mut self, permit: Option<OwnedSemaphorePermit>) -> Self {
        self.permit = permit;
        self
    }

    /// Builds the [`ConnectionGuard`] which should be handed to the connection task and the [`ConnectionHandle`].
    ///
    /// This will panic if a permit was not set [`HandleBuilder::with_permit`]
    pub fn build(self) -> (ConnectionGuard, ConnectionHandle) {
        let token = CancellationToken::new();

        (
            ConnectionGuard {
                token: token.clone(),
                _permit: self.permit,
            },
            ConnectionHandle {
                token,
                ban: Arc::new(OnceLock::new()),
            },
        )
    }
}

/// A struct representing the time a peer should be banned for.
#[derive(Debug, Copy, Clone)]
pub struct BanPeer(pub Duration);

/// A struct given to the connection task.
pub struct ConnectionGuard {
    token: CancellationToken,
    _permit: Option<OwnedSemaphorePermit>,
}

impl ConnectionGuard {
    /// Checks if we should close the connection.
    pub fn should_shutdown(&self) -> WaitForCancellationFutureOwned {
        self.token.clone().cancelled_owned()
    }
    /// Tell the corresponding [`ConnectionHandle`]s that this connection is closed.
    ///
    /// This will be called on [`Drop::drop`].
    pub fn connection_closed(&self) {
        self.token.cancel();
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.token.cancel();
    }
}

/// A handle given to a task that needs to ban, disconnect, check if the peer should be banned or check
/// the peer is still connected.
#[derive(Debug, Clone)]
pub struct ConnectionHandle {
    token: CancellationToken,
    ban: Arc<OnceLock<BanPeer>>,
}

impl ConnectionHandle {
    pub fn closed(&self) -> WaitForCancellationFutureOwned {
        self.token.clone().cancelled_owned()
    }
    /// Bans the peer for the given `duration`.
    pub fn ban_peer(&self, duration: Duration) {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "error means peer is already banned; fine to ignore"
        )]
        let _ = self.ban.set(BanPeer(duration));
        self.token.cancel();
    }
    /// Checks if this connection is closed.
    pub fn is_closed(&self) -> bool {
        self.token.is_cancelled()
    }
    /// Returns if this peer has been banned and the [`Duration`] of that ban.
    pub fn check_should_ban(&mut self) -> Option<BanPeer> {
        self.ban.get().copied()
    }
    /// Sends the signal to the connection task to disconnect.
    pub fn send_close_signal(&self) {
        self.token.cancel();
    }
}

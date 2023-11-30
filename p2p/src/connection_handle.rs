//!
//! # Why do we need a handle between the address book and connection task
//!
//! When banning a peer we need to tell the connection task to close and
//! when we close a connection we need to tell the address book.
//!
//!
use std::time::Duration;

use futures::channel::mpsc;
use futures::SinkExt;
use tokio_util::sync::CancellationToken;

use crate::connection_counter::ConnectionTracker;

#[derive(Default, Debug)]
pub struct HandleBuilder {
    tracker: Option<ConnectionTracker>,
}

impl HandleBuilder {
    pub fn set_tracker(&mut self, tracker: ConnectionTracker) {
        self.tracker = Some(tracker)
    }

    pub fn build(self) -> (DisconnectSignal, ConnectionHandle, PeerHandle) {
        let token = CancellationToken::new();
        let (tx, rx) = mpsc::channel(0);

        (
            DisconnectSignal {
                token: token.clone(),
                tracker: self.tracker.expect("Tracker was not set!"),
            },
            ConnectionHandle {
                token: token.clone(),
                ban: rx,
            },
            PeerHandle { ban: tx },
        )
    }
}

pub struct BanPeer(pub Duration);

/// A struct given to the connection task.
pub struct DisconnectSignal {
    token: CancellationToken,
    tracker: ConnectionTracker,
}

impl DisconnectSignal {
    pub fn should_shutdown(&self) -> bool {
        self.token.is_cancelled()
    }
    pub fn connection_closed(&self) {
        self.token.cancel()
    }
}

impl Drop for DisconnectSignal {
    fn drop(&mut self) {
        self.token.cancel()
    }
}

/// A handle given to a task that needs to cancel this connection.
pub struct ConnectionHandle {
    token: CancellationToken,
    ban: mpsc::Receiver<BanPeer>,
}

impl ConnectionHandle {
    pub fn is_closed(&self) -> bool {
        self.token.is_cancelled()
    }
    pub fn check_should_ban(&mut self) -> Option<BanPeer> {
        match self.ban.try_next() {
            Ok(res) => res,
            Err(_) => None,
        }
    }
    pub fn send_close_signal(&self) {
        self.token.cancel()
    }
}

/// A handle given to a task that needs to be able to ban a connection.
#[derive(Clone)]
pub struct PeerHandle {
    ban: mpsc::Sender<BanPeer>,
}

impl PeerHandle {
    pub fn ban_peer(&mut self, duration: Duration) {
        let _ = self.ban.send(BanPeer(duration));
    }
}

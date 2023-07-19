//! This module contains the address book [`Connection`](crate::peer::connection::Connection) handle
//!
//! # Why do we need a handle between the address book and connection task
//!
//! When banning a peer we need to tell the connection task to close and
//! when we close a connection we need to remove it from our connection
//! and anchor list.
//!
//!
use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, SinkExt, StreamExt};

/// A message sent to tell the address book that a peer has disconnected.
pub struct PeerConnectionClosed(Option<std::time::Duration>);

pub enum ConnectionClosed {
    Closed(Option<std::time::Duration>),
    Running,
}

/// The connection side of the address book to connection
/// communication.
#[derive(Debug)]
pub struct ConnectionHandleAddressBookSide {
    connection_closed_rx: oneshot::Receiver<PeerConnectionClosed>,
    ban_tx: mpsc::Sender<std::time::Duration>,
}

impl ConnectionHandleAddressBookSide {
    pub fn ban_peer(&mut self, duration: std::time::Duration) {
        let _ = self.ban_tx.send(duration);
    }

    pub fn check_connection_closed(&mut self) -> ConnectionClosed {
        let connection_closed = self
            .connection_closed_rx
            .try_recv()
            .expect("Will not be cancelled");
        match connection_closed {
            Some(closed) => return ConnectionClosed::Closed(closed.0),
            None => ConnectionClosed::Running,
        }
    }
}

/// The address book side of the address book to connection
/// communication.
#[derive(Debug)]
pub struct ConnectionHandleConnectionSide {
    connection_closed_tx: Option<oneshot::Sender<PeerConnectionClosed>>,
    ban_rx: mpsc::Receiver<std::time::Duration>,
    ban_holder: Option<std::time::Duration>,
}

impl ConnectionHandleConnectionSide {
    pub fn been_banned(&mut self) -> bool {
        let ban_time =
            self.ban_rx.next().now_or_never().and_then(|inner| {
                Some(inner.expect("Handles to the connection task wont be dropped"))
            });
        let ret = ban_time.is_some();
        self.ban_holder = ban_time;
        ret
    }
}

impl Drop for ConnectionHandleConnectionSide {
    fn drop(&mut self) {
        let tx = std::mem::replace(&mut self.connection_closed_tx, None)
            .expect("Will only be dropped once");
        let _ = tx.send(PeerConnectionClosed(self.ban_holder));
    }
}

pub struct ConnectionHandleClientSide {
    ban_tx: mpsc::Sender<std::time::Duration>,
}

impl ConnectionHandleClientSide {
    pub fn ban_peer(&mut self, duration: std::time::Duration) {
        let _ = self.ban_tx.send(duration);
    }
}

/// Creates a new handle pair that can be given to the connection task and
/// address book respectively.
pub fn new_address_book_connection_handle() -> (
    ConnectionHandleConnectionSide,
    ConnectionHandleAddressBookSide,
    ConnectionHandleClientSide,
) {
    let (tx_closed, rx_closed) = oneshot::channel();
    let (tx_ban, rx_ban) = mpsc::channel(0);

    let c_h_c_s = ConnectionHandleConnectionSide {
        connection_closed_tx: Some(tx_closed),
        ban_rx: rx_ban,
        ban_holder: None,
    };
    let c_h_a_s = ConnectionHandleAddressBookSide {
        connection_closed_rx: rx_closed,
        ban_tx: tx_ban.clone(),
    };
    let c_h_cl_s = ConnectionHandleClientSide { ban_tx: tx_ban };

    (c_h_c_s, c_h_a_s, c_h_cl_s)
}

#[cfg(test)]
mod tests {
    use super::new_address_book_connection_handle;

    #[test]
    fn close_connection_from_address_book() {
        let (conn_side, mut addr_side) = new_address_book_connection_handle();

        assert!(!conn_side.is_canceled());
        assert!(!addr_side.connection_closed());
        addr_side.kill_connection();
        assert!(conn_side.is_canceled());
    }

    #[test]
    fn close_connection_from_connection() {
        let (conn_side, mut addr_side) = new_address_book_connection_handle();

        assert!(!conn_side.is_canceled());
        assert!(!addr_side.connection_closed());
        drop(conn_side);
        assert!(addr_side.connection_closed());
    }
}

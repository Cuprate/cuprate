//! This module contains the address book [`Connection`](crate::peer::connection::Connection) handle
//!
//! # Why do we need a handle between the address book and connection task
//!
//! When banning a peer we need to tell the connection task to close and
//! when we close a connection we need to remove it from our connection
//! and anchor list.
//!
//!
use futures::channel::oneshot;
use tokio_util::sync::CancellationToken;

/// A message sent to tell the address book that a peer has disconnected.
pub struct PeerConnectionClosed;

/// The connection side of the address book to connection
/// communication.
#[derive(Debug)]
pub struct AddressBookConnectionHandle {
    connection_closed: oneshot::Sender<PeerConnectionClosed>,
    close: CancellationToken,
}

impl AddressBookConnectionHandle {
    /// Tell the address book the connection has closed.
    pub fn connection_closed(self) {
        let _ = self.connection_closed.send(PeerConnectionClosed);
    }
    /// Returns true if the address book has told us to kill the
    /// connection.
    pub fn is_canceled(&self) -> bool {
        self.close.is_cancelled()
    }
}

/// The address book side of the address book to connection
/// communication.
#[derive(Debug)]
pub struct ConnectionAddressBookHandle {
    connection_closed: oneshot::Receiver<PeerConnectionClosed>,
    killer: CancellationToken,
}

impl ConnectionAddressBookHandle {
    /// Checks if the connection task has closed, returns
    /// true if the task has closed
    pub fn connection_closed(&mut self) -> bool {
        let Ok(mes) = self.connection_closed.try_recv() else {
            panic!("This must not be called again after returning true and the connection task must tell us if a connection is closed")
        };
        match mes {
            None => false,
            Some(_) => true,
        }
    }

    /// Ends the connection task, the caller of this function should
    /// wait to be told the connection has closed by [`check_if_connection_closed`](Self::check_if_connection_closed)
    /// before acting on the closed connection.
    pub fn kill_connection(&self) {
        self.killer.cancel()
    }
}

/// Creates a new handle pair that can be given to the connection task and
/// address book respectively.
pub fn new_address_book_connection_handle(
) -> (AddressBookConnectionHandle, ConnectionAddressBookHandle) {
    let (tx, rx) = oneshot::channel();
    let token = CancellationToken::new();

    let ab_c_h = AddressBookConnectionHandle {
        connection_closed: tx,
        close: token.clone(),
    };
    let c_ab_h = ConnectionAddressBookHandle {
        connection_closed: rx,
        killer: token,
    };

    (ab_c_h, c_ab_h)
}

#[cfg(test)]
mod tests {
    use crate::address_book::connection_handle::new_address_book_connection_handle;

    fn close_connection_from_address_book() {
        let (conn_side, mut addr_side) = new_address_book_connection_handle();

        assert!(!conn_side.is_canceled());
        assert!(!addr_side.connection_closed());
        addr_side.kill_connection();
        assert!(conn_side.is_canceled());
    }

    fn close_connection_from_connection() {
        let (conn_side, mut addr_side) = new_address_book_connection_handle();

        assert!(!conn_side.is_canceled());
        assert!(!addr_side.connection_closed());
        conn_side.connection_closed();
        assert!(addr_side.connection_closed());
    }
}

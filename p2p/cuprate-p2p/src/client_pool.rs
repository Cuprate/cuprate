//! # Client Pool.
//!
//! The [`ClientPool`], is a pool of currently connected peers that can be pulled from.
//! It does _not_ necessarily contain every connected peer as another place could have
//! taken a peer from the pool.
//!
//! When taking peers from the pool they are wrapped in [`ClientPoolDropGuard`], which
//! returns the peer to the pool when it is dropped.
//!
//! Internally the pool is a [`DashMap`] which means care should be taken in `async` code
//! as internally this uses blocking RwLocks.
//!
use std::sync::Arc;

use dashmap::{DashMap, DashSet};
use tokio::sync::mpsc;

use monero_p2p::{
    client::{Client, InternalPeerID},
    ConnectionDirection, NetworkZone,
};

mod disconnect_monitor;
mod drop_guard_client;

pub use drop_guard_client::ClientPoolDropGuard;
use monero_p2p::handles::ConnectionHandle;

/// The client pool, which holds currently connected free peers.
///
/// See the [module docs](self) for more.
pub struct ClientPool<N: NetworkZone> {
    /// The connected [`Client`]s.
    clients: DashMap<InternalPeerID<N::Addr>, Client<N>>,
    /// A set of outbound clients, as these allow accesses/ mutation from different threads
    /// a peer ID in here does not mean the peer is definitely in `clients` , if the peer is
    /// in both here and `clients` it is defiantly an outbound peer,
    outbound_clients: DashSet<InternalPeerID<N::Addr>>,

    /// A channel to send new peer ids down to monitor for disconnect.
    new_connection_tx: mpsc::UnboundedSender<(ConnectionHandle, InternalPeerID<N::Addr>)>,
}

impl<N: NetworkZone> ClientPool<N> {
    pub fn new() -> Arc<ClientPool<N>> {
        let (tx, rx) = mpsc::unbounded_channel();

        let pool = Arc::new(ClientPool {
            clients: DashMap::new(),
            outbound_clients: DashSet::new(),
            new_connection_tx: tx,
        });

        tokio::spawn(disconnect_monitor::disconnect_monitor(rx, pool.clone()));

        pool
    }

    fn add_client(&self, client: Client<N>) {
        let handle = client.info.handle.clone();
        let id = client.info.id;

        if handle.is_closed() {
            return;
        }

        if client.info.direction == ConnectionDirection::OutBound {
            self.outbound_clients.insert(id);
        }

        let res = self.clients.insert(id, client);
        debug_assert!(res.is_none());

        // TODO: document how this prevents a race condition.
        if handle.is_closed() {
            self.remove_client(&id);
        }
    }

    pub fn add_new_client(&self, client: Client<N>) {
        self.new_connection_tx
            .send((client.info.handle.clone(), client.info.id))
            .unwrap();

        self.add_client(client);
    }

    fn remove_client(&self, peer: &InternalPeerID<N::Addr>) -> Option<Client<N>> {
        self.outbound_clients.remove(peer);

        self.clients.remove(peer).map(|(_, client)| client)
    }

    pub fn borrow_client(
        self: &Arc<Self>,
        peer: &InternalPeerID<N::Addr>,
    ) -> Option<ClientPoolDropGuard<N>> {
        self.outbound_clients.remove(peer);

        self.remove_client(peer).map(|client| ClientPoolDropGuard {
            pool: Arc::clone(self),
            client: Some(client),
        })
    }

    pub fn borrow_clients(
        self: &Arc<Self>,
        peers: &[InternalPeerID<N::Addr>],
    ) -> Vec<ClientPoolDropGuard<N>> {
        peers
            .iter()
            .filter_map(|peer| self.borrow_client(peer))
            .collect()
    }
}

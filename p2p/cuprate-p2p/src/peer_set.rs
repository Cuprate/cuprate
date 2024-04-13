//! This module contains the peer set and related functionality.
//!
use std::sync::Arc;

use dashmap::{DashMap, DashSet};
use futures::FutureExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tower::ServiceExt;

use monero_p2p::{
    client::{Client, InternalPeerID},
    ConnectionDirection, NetworkZone,
};

mod disconnect_monitor;
mod drop_guard_client;

pub use drop_guard_client::ClientPoolGuard;
use monero_p2p::handles::ConnectionHandle;

pub struct ClientPool<N: NetworkZone> {
    clients: DashMap<InternalPeerID<N::Addr>, Client<N>>,
    outbound_clients: DashSet<InternalPeerID<N::Addr>>,

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

    fn add_client(&self, mut client: Client<N>) {
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
    ) -> Option<ClientPoolGuard<N>> {
        self.outbound_clients.remove(peer);

        self.remove_client(peer).map(|client| ClientPoolGuard {
            pool: Arc::clone(self),
            client: Some(client),
        })
    }

    pub fn borrow_clients(
        self: &Arc<Self>,
        peers: &[InternalPeerID<N::Addr>],
    ) -> Vec<ClientPoolGuard<N>> {
        peers
            .iter()
            .filter_map(|peer| self.borrow_client(peer))
            .collect()
    }
}

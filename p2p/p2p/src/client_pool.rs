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
//! as internally this uses blocking `RwLock`s.
use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::mpsc;
use tracing::{Instrument, Span};

use cuprate_p2p_core::{
    client::{Client, InternalPeerID},
    handles::ConnectionHandle,
    NetworkZone,
};

pub(crate) mod disconnect_monitor;
mod drop_guard_client;

pub(crate) use drop_guard_client::ClientPoolDropGuard;

/// The client pool, which holds currently connected free peers.
///
/// See the [module docs](self) for more.
pub struct ClientPool<N: NetworkZone> {
    /// The connected [`Client`]s.
    clients: DashMap<InternalPeerID<N::Addr>, Client<N>>,
    /// A channel to send new peer ids down to monitor for disconnect.
    new_connection_tx: mpsc::UnboundedSender<(ConnectionHandle, InternalPeerID<N::Addr>)>,
}

impl<N: NetworkZone> ClientPool<N> {
    /// Returns a new [`ClientPool`] wrapped in an [`Arc`].
    pub fn new() -> Arc<Self> {
        let (tx, rx) = mpsc::unbounded_channel();

        let pool = Arc::new(Self {
            clients: DashMap::new(),
            new_connection_tx: tx,
        });

        tokio::spawn(
            disconnect_monitor::disconnect_monitor(rx, Arc::clone(&pool))
                .instrument(Span::current()),
        );

        pool
    }

    /// Adds a [`Client`] to the pool, the client must have previously been taken from the
    /// pool.
    ///
    /// See [`ClientPool::add_new_client`] to add a [`Client`] which was not taken from the pool before.
    ///
    /// # Panics
    /// This function panics if `client` already exists in the pool.
    fn add_client(&self, client: Client<N>) {
        let handle = client.info.handle.clone();
        let id = client.info.id;

        // Fast path: if the client is disconnected don't add it to the peer set.
        if handle.is_closed() {
            return;
        }

        assert!(self.clients.insert(id, client).is_none());

        // We have to check this again otherwise we could have a race condition where a
        // peer is disconnected after the first check, the disconnect monitor tries to remove it,
        // and then it is added to the pool.
        if handle.is_closed() {
            self.remove_client(&id);
        }
    }

    /// Adds a _new_ [`Client`] to the pool, this client should be a new connection, and not already
    /// from the pool.
    ///
    /// # Panics
    /// This function panics if `client` already exists in the pool.
    pub fn add_new_client(&self, client: Client<N>) {
        self.new_connection_tx
            .send((client.info.handle.clone(), client.info.id))
            .unwrap();

        self.add_client(client);
    }

    /// Remove a [`Client`] from the pool.
    ///
    /// [`None`] is returned if the client did not exist in the pool.
    fn remove_client(&self, peer: &InternalPeerID<N::Addr>) -> Option<Client<N>> {
        self.clients.remove(peer).map(|(_, client)| client)
    }

    /// Borrows a [`Client`] from the pool.
    ///
    /// The [`Client`] is wrapped in [`ClientPoolDropGuard`] which
    /// will return the client to the pool when it's dropped.
    ///
    /// See [`Self::borrow_clients`] for borrowing multiple clients.
    pub fn borrow_client(
        self: &Arc<Self>,
        peer: &InternalPeerID<N::Addr>,
    ) -> Option<ClientPoolDropGuard<N>> {
        self.remove_client(peer).map(|client| ClientPoolDropGuard {
            pool: Arc::clone(self),
            client: Some(client),
        })
    }

    /// Borrows multiple [`Client`]s from the pool.
    ///
    /// Note that the returned iterator is not guaranteed to contain every peer asked for.
    ///
    /// See [`Self::borrow_client`] for borrowing a single client.
    pub fn borrow_clients<'a, 'b>(
        self: &'a Arc<Self>,
        peers: &'b [InternalPeerID<N::Addr>],
    ) -> impl Iterator<Item = ClientPoolDropGuard<N>> + sealed::Captures<(&'a (), &'b ())> {
        peers.iter().filter_map(|peer| self.borrow_client(peer))
    }

    /// Borrows all [`Client`]s from the pool that have claimed a higher cumulative difficulty than
    /// the amount passed in.
    ///
    /// The [`Client`]s are wrapped in [`ClientPoolDropGuard`] which
    /// will return the clients to the pool when they are dropped.
    pub fn clients_with_more_cumulative_difficulty(
        self: &Arc<Self>,
        cumulative_difficulty: u128,
    ) -> Vec<ClientPoolDropGuard<N>> {
        let peers = self
            .clients
            .iter()
            .filter_map(|element| {
                let peer_sync_info = element.value().info.core_sync_data.lock().unwrap();

                if peer_sync_info.cumulative_difficulty() > cumulative_difficulty {
                    Some(*element.key())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        self.borrow_clients(&peers).collect()
    }

    pub fn contains_client_with_more_cumulative_difficulty(
        &self,
        cumulative_difficulty: u128,
    ) -> bool {
        self.clients.iter().any(|element| {
            let sync_data = element.value().info.core_sync_data.lock().unwrap();
            sync_data.cumulative_difficulty() > cumulative_difficulty
        })
    }
}

mod sealed {
    /// TODO: Remove me when 2024 Rust
    ///
    /// <https://rust-lang.github.io/rfcs/3498-lifetime-capture-rules-2024.html#the-captures-trick>
    pub trait Captures<U> {}

    impl<T: ?Sized, U> Captures<U> for T {}
}

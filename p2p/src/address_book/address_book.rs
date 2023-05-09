//! This module contains the actual address book logic.
//!
//! The address book is split into multiple [`PeerList`]:
//!
//! - A White list: For peers we have connected to ourselves,
//!
//! - A Gray list: For Peers we have been told about but
//!   haven't connected to ourselves.
//!
//! - An Anchor list: This holds peers we are currently
//!   connected to that are reachable if we were to
//!   connect to them again. For example an inbound proxy
//!   connection would not get added to this list as we cant
//!   connect to this peer ourselves. Behind the scenes we
//!   are just storing the key to a peer in the whit list.
//!
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::stream::FuturesUnordered;
use futures::{
    channel::{mpsc, oneshot},
    FutureExt, Stream, StreamExt,
};
use pin_project::pin_project;
use rand::prelude::SliceRandom;

use cuprate_common::shutdown::is_shutting_down;
use cuprate_common::PruningSeed;
use monero_wire::{messages::PeerListEntryBase, network_address::NetZone, NetworkAddress, PeerID};

use super::{AddressBookError, AddressBookRequest, AddressBookResponse};
use crate::address_book::connection_handle::ConnectionAddressBookHandle;
use crate::{constants::ADDRESS_BOOK_SAVE_INTERVAL, Config, P2PStore};

mod peer_list;
use peer_list::PeerList;

#[cfg(test)]
mod tests;

/// A request sent to the address book task.
pub(crate) struct AddressBookClientRequest {
    /// The request
    pub req: AddressBookRequest,
    /// A oneshot to send the result down
    pub tx: oneshot::Sender<Result<AddressBookResponse, AddressBookError>>,
    /// The tracing span to keep the context of the request
    pub span: tracing::Span,
}

/// An entry in the connected list.
pub struct ConnectionPeerEntry {
    /// A oneshot sent from the Connection when it has finished.
    connection_handle: ConnectionAddressBookHandle,
    /// The connection addr, None if the peer is connected through
    /// a hidden network.
    addr: Option<NetworkAddress>,
    /// If the peer is reachable by our node.
    reachable: bool,
    /// The last seen timestamp, note: Cuprate may skip updating this
    /// field on some inbound messages
    last_seen: chrono::NaiveDateTime,
    /// The peers pruning seed
    pruning_seed: PruningSeed,
    /// The peers port.
    rpc_port: u16,
    /// The peers rpc credits per hash
    rpc_credits_per_hash: u32,
}

/// A future that resolves when a peer is unbanned.
#[pin_project(project = EnumProj)]
pub struct BanedPeerFut(Vec<u8>, #[pin] tokio::time::Sleep);

impl Future for BanedPeerFut {
    type Output = Vec<u8>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        match this.1.poll_unpin(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(_) => Poll::Ready(this.0.clone()),
        }
    }
}

/// The address book for a specific [`NetZone`]
pub struct AddressBook<PeerStore> {
    /// The [`NetZone`] of this address book.
    zone: NetZone,
    /// A copy of the nodes configuration.
    config: Config,
    /// The Address books white list.
    white_list: PeerList,
    /// The Address books gray list.
    gray_list: PeerList,
    /// The Address books anchor list.
    anchor_list: HashSet<NetworkAddress>,
    /// The Currently connected peers.
    connected_peers: HashMap<PeerID, ConnectionPeerEntry>,
    /// A tuple of:
    /// - A hashset of [`ban_identifier`](NetworkAddress::ban_identifier)
    /// - A [`FuturesUnordered`] which contains futures for every ban_id
    ///   that will resolve when the ban_id should be un banned.
    baned_peers: (HashSet<Vec<u8>>, FuturesUnordered<BanedPeerFut>),
    /// The peer store to save the peers to persistent storage
    p2p_store: PeerStore,
}

impl<PeerStore: P2PStore> AddressBook<PeerStore> {
    /// Creates a new address book for a given [`NetZone`]
    pub fn new(
        config: Config,
        zone: NetZone,
        white_peers: Vec<PeerListEntryBase>,
        gray_peers: Vec<PeerListEntryBase>,
        anchor_peers: Vec<NetworkAddress>,
        baned_peers: Vec<(NetworkAddress, chrono::NaiveDateTime)>,
        p2p_store: PeerStore,
    ) -> Self {
        let white_list = PeerList::new(white_peers);
        let gray_list = PeerList::new(gray_peers);
        let anchor_list = HashSet::from_iter(anchor_peers);
        let baned_peers = (HashSet::new(), FuturesUnordered::new());

        let connected_peers = HashMap::new();

        AddressBook {
            zone,
            config,
            white_list,
            gray_list,
            anchor_list,
            connected_peers,
            baned_peers,
            p2p_store,
        }
    }

    /// Returns the books name (Based on the [`NetZone`])
    pub const fn book_name(&self) -> &'static str {
        match self.zone {
            NetZone::Public => "PublicAddressBook",
            NetZone::Tor => "TorAddressBook",
            NetZone::I2p => "I2pAddressBook",
        }
    }

    /// Returns the length of the white list
    fn len_white_list(&self) -> usize {
        self.white_list.len()
    }

    /// Returns the length of the gray list
    fn len_gray_list(&self) -> usize {
        self.gray_list.len()
    }

    /// Returns the length of the anchor list
    fn len_anchor_list(&self) -> usize {
        self.anchor_list.len()
    }

    /// Returns the length of the banned list
    fn len_banned_list(&self) -> usize {
        self.baned_peers.0.len()
    }

    /// Returns the maximum length of the white list
    /// *note this list can grow bigger if we are connected to more
    /// than this amount.
    fn max_white_peers(&self) -> usize {
        self.config.max_white_peers()
    }

    /// Returns the maximum length of the gray list
    fn max_gray_peers(&self) -> usize {
        self.config.max_gray_peers()
    }

    /// Checks if a peer is banned.
    fn is_peer_banned(&self, peer: &NetworkAddress) -> bool {
        self.baned_peers.0.contains(&peer.ban_identifier())
    }

    /// Checks if banned peers should be unbanned as the duration has elapsed
    fn check_unban_peers(&mut self) {
        while let Some(Some(addr)) = Pin::new(&mut self.baned_peers.1).next().now_or_never() {
            tracing::debug!("Unbanning peer: {addr:?}");
            self.baned_peers.0.remove(&addr);
        }
    }

    /// Checks if peers have disconnected, if they have removing them from the
    /// connected and anchor list.
    fn check_connected_peers(&mut self) {
        let mut remove_from_anchor = vec![];
        // We dont have to worry about updating our white list with the information
        // before we remove the peers as that happens on every save.
        self.connected_peers.retain(|_, peer| {
            if !peer.connection_handle.connection_closed() {
                // add the peer to the list to get removed from the anchor
                if let Some(addr) = peer.addr {
                    remove_from_anchor.push(addr)
                }
                false
            } else {
                true
            }
        });
        // If we are shutting down we want to keep our anchor peers for
        // the next time we boot up so we dont remove disconnecting peers
        // from the anchor list if we are shutting down.
        if !is_shutting_down() {
            for peer in remove_from_anchor {
                self.anchor_list.remove(&peer);
            }
        }
    }

    // Bans the peer and tells the connection tasks of peers with the same ban id to shutdown.
    fn ban_peer(
        &mut self,
        peer: PeerID,
        time: std::time::Duration,
    ) -> Result<(), AddressBookError> {
        tracing::debug!("Banning peer: {peer:?} for: {time:?}");

        let Some(conn_entry) = self.connected_peers.get(&peer) else {
            tracing::debug!("Peer is not in connected list");
            return Err(AddressBookError::PeerNotFound);
        };
        // tell the connection task to finish.
        conn_entry.connection_handle.kill_connection();
        // try find the NetworkAddress of the peer
        let Some(addr) = conn_entry.addr else {
            tracing::debug!("Peer does not have an address we can ban");
            return Ok(());
        };

        let ban_id = addr.ban_identifier();

        self.white_list.remove_peers_with_ban_id(&ban_id);
        self.gray_list.remove_peers_with_ban_id(&ban_id);
        // Dont remove from anchor list or connection list as this will happen when
        // the connection is closed.

        // tell the connection task of peers with the same ban id to shutdown.
        for conn in self.connected_peers.values() {
            if let Some(addr) = conn.addr {
                if addr.ban_identifier() == ban_id {
                    conn.connection_handle.kill_connection()
                }
            }
        }

        // add the ban identifier to the ban list
        self.baned_peers.0.insert(ban_id.clone());
        self.baned_peers
            .1
            .push(BanedPeerFut(ban_id, tokio::time::sleep(time)));
        Ok(())
    }

    /// Update the last seen timestamp of a connected peer.
    fn update_last_seen(
        &mut self,
        peer: PeerID,
        last_seen: chrono::NaiveDateTime,
    ) -> Result<(), AddressBookError> {
        if let Some(mut peer) = self.connected_peers.get_mut(&peer) {
            peer.last_seen = last_seen;
            Ok(())
        } else {
            Err(AddressBookError::PeerNotFound)
        }
    }

    /// adds a peer to the gray list.
    fn add_peer_to_gray_list(&mut self, mut peer: PeerListEntryBase) {
        if self.white_list.contains_peer(&peer.adr) {
            return;
        };
        if !self.gray_list.contains_peer(&peer.adr) {
            peer.last_seen = 0;
            self.gray_list.add_new_peer(peer);
        }
    }

    /// handles an incoming peer list,
    /// dose some basic validation on the addresses
    /// appends the good peers to our book.
    fn handle_new_peerlist(
        &mut self,
        mut peers: Vec<PeerListEntryBase>,
    ) -> Result<(), AddressBookError> {
        let length = peers.len();

        tracing::debug!("Received new peer list, length: {length}");

        let mut err = None;
        peers.retain(|peer| {
            if err.is_some() {
                false
            } else if peer.adr.is_local() || peer.adr.is_loopback() {
                false
            } else if peer.adr.port() == peer.rpc_port {
                false
            } else if PruningSeed::try_from(peer.pruning_seed).is_err() {
                false
            } else if peer.adr.get_zone() != self.zone {
                tracing::info!("Received an address from a different network zone, ignoring list.");
                err = Some(AddressBookError::PeerSentAnAddressOutOfZone);
                false
            } else if self.is_peer_banned(&peer.adr) {
                false
            } else {
                true
            }
        });

        if let Some(e) = err {
            return Err(e);
        } else {
            for peer in peers {
                self.add_peer_to_gray_list(peer);
            }
            self.gray_list
                .reduce_list(&HashSet::new(), self.max_gray_peers());
            Ok(())
        }
    }

    /// Gets a random peer from our gray list.
    /// If pruning seed is set we will get a peer with that pruning seed.
    fn get_random_gray_peer(
        &mut self,
        pruning_seed: Option<PruningSeed>,
    ) -> Option<PeerListEntryBase> {
        self.gray_list
            .get_random_peer(&mut rand::thread_rng(), pruning_seed.map(Into::into))
            .map(|p| *p)
    }

    /// Gets a random peer from our white list.
    /// If pruning seed is set we will get a peer with that pruning seed.
    fn get_random_white_peer(
        &mut self,
        pruning_seed: Option<PruningSeed>,
    ) -> Option<PeerListEntryBase> {
        self.white_list
            .get_random_peer(&mut rand::thread_rng(), pruning_seed.map(Into::into))
            .map(|p| *p)
    }

    /// Gets random peers from our white list.
    /// will be less than or equal to `len`.
    fn get_random_white_peers(&mut self, len: usize) -> Vec<PeerListEntryBase> {
        let white_len = self.white_list.len();
        let len = if len < white_len { len } else { white_len };
        let mut white_peers: Vec<&PeerListEntryBase> = self.white_list.iter_all_peers().collect();
        white_peers.shuffle(&mut rand::thread_rng());
        white_peers[0..len].iter().map(|peb| **peb).collect()
    }

    /// Updates an entry in the white list, if the peer is not found and `reachable` is true then
    /// the peer will be added to the white list.
    fn update_white_list_peer_entry(
        &mut self,
        addr: &NetworkAddress,
        id: PeerID,
        conn_entry: &ConnectionPeerEntry,
    ) -> Result<(), AddressBookError> {
        if let Some(peb) = self.white_list.get_peer_mut(addr) {
            if peb.pruning_seed == conn_entry.pruning_seed.into() {
                return Err(AddressBookError::PeersPruningSeedChanged);
            }
            peb.id = id;
            peb.last_seen = conn_entry.last_seen.timestamp();
            peb.rpc_port = conn_entry.rpc_port;
            peb.rpc_credits_per_hash = conn_entry.rpc_credits_per_hash;
            peb.pruning_seed = conn_entry.pruning_seed.into();
        } else if conn_entry.reachable {
            // if the peer is reachable add it to our white list
            let peb = PeerListEntryBase {
                id,
                adr: *addr,
                last_seen: conn_entry.last_seen.timestamp(),
                rpc_port: conn_entry.rpc_port,
                rpc_credits_per_hash: conn_entry.rpc_credits_per_hash,
                pruning_seed: conn_entry.pruning_seed.into(),
            };
            self.white_list.add_new_peer(peb);
        }
        Ok(())
    }

    /// Handles a new connection, adding it to the white list if the
    /// peer is reachable by our node.
    fn handle_new_connection(
        &mut self,
        connection_handle: ConnectionAddressBookHandle,
        addr: Option<NetworkAddress>,
        id: PeerID,
        reachable: bool,
        last_seen: chrono::NaiveDateTime,
        pruning_seed: PruningSeed,
        rpc_port: u16,
        rpc_credits_per_hash: u32,
    ) -> Result<(), AddressBookError> {
        let connection_entry = ConnectionPeerEntry {
            connection_handle,
            addr,
            reachable,
            last_seen,
            pruning_seed,
            rpc_port,
            rpc_credits_per_hash,
        };
        if let Some(addr) = addr {
            if self.baned_peers.0.contains(&addr.ban_identifier()) {
                return Err(AddressBookError::PeerIsBanned);
            }
            // remove the peer from the gray list as we know it's active.
            let _ = self.gray_list.remove_peer(&addr);
            if !reachable {
                // If we can't reach the peer remove it from the white list as well
                let _ = self.white_list.remove_peer(&addr);
            } else {
                // The peer is reachable, update our white list and add it to the anchor connections.
                self.update_white_list_peer_entry(&addr, id, &connection_entry)?;
                self.anchor_list.insert(addr);
            }
        }

        self.connected_peers.insert(id, connection_entry);
        self.white_list
            .reduce_list(&self.anchor_list, self.max_white_peers());
        Ok(())
    }

    /// Get and empties the anchor list, used at startup to
    /// connect to some peers we were previously connected to.
    fn get_and_empty_anchor_list(&mut self) -> Vec<PeerListEntryBase> {
        self.anchor_list
            .drain()
            .map(|addr| {
                self.white_list
                    .get_peer(&addr)
                    .expect("If peer is in anchor it must be in white list")
                    .clone()
            })
            .collect()
    }

    /// Handles an [`AddressBookClientRequest`] to the address book.
    async fn handle_request(&mut self, req: AddressBookClientRequest) {
        let _guard = req.span.enter();

        tracing::trace!("received request: {}", req.req);

        let res = match req.req {
            AddressBookRequest::HandleNewPeerList(new_peers, _) => self
                .handle_new_peerlist(new_peers)
                .map(|_| AddressBookResponse::Ok),
            AddressBookRequest::SetPeerSeen(peer, last_seen, _) => self
                .update_last_seen(peer, last_seen)
                .map(|_| AddressBookResponse::Ok),
            AddressBookRequest::BanPeer(peer, time, _) => {
                self.ban_peer(peer, time).map(|_| AddressBookResponse::Ok)
            }
            AddressBookRequest::ConnectedToPeer {
                zone: _,
                connection_handle,
                addr,
                id,
                reachable,
                last_seen,
                pruning_seed,
                rpc_port,
                rpc_credits_per_hash,
            } => self
                .handle_new_connection(
                    connection_handle,
                    addr,
                    id,
                    reachable,
                    last_seen,
                    pruning_seed,
                    rpc_port,
                    rpc_credits_per_hash,
                )
                .map(|_| AddressBookResponse::Ok),

            AddressBookRequest::GetAndEmptyAnchorList(_) => {
                Ok(AddressBookResponse::Peers(self.get_and_empty_anchor_list()))
            }

            AddressBookRequest::GetRandomGrayPeer(_, pruning_seed) => {
                match self.get_random_gray_peer(pruning_seed) {
                    Some(peer) => Ok(AddressBookResponse::Peer(peer)),
                    None => Err(AddressBookError::PeerListEmpty),
                }
            }
            AddressBookRequest::GetRandomWhitePeer(_, pruning_seed) => {
                match self.get_random_white_peer(pruning_seed) {
                    Some(peer) => Ok(AddressBookResponse::Peer(peer)),
                    None => Err(AddressBookError::PeerListEmpty),
                }
            }
            AddressBookRequest::GetRandomWhitePeers(_, len) => {
                Ok(AddressBookResponse::Peers(self.get_random_white_peers(len)))
            }
        };

        if let Err(e) = &res {
            tracing::debug!("Error when handling request, err: {e}")
        }

        let _ = req.tx.send(res);
    }

    /// Updates the white list with the information in the `connected_peers` list.
    /// This only updates the `last_seen` timestamp as that's the only thing that should
    /// change during connections.
    fn update_white_list_with_conn_list(&mut self) {
        for (_, peer) in self.connected_peers.iter() {
            if peer.reachable {
                if let Some(peer_eb) = self.white_list.get_peer_mut(&peer.addr.unwrap()) {
                    peer_eb.last_seen = peer.last_seen.timestamp();
                }
            }
        }
    }

    /// Saves the address book to persistent storage.
    /// TODO: save the banned peer list.
    #[tracing::instrument(level="trace", skip(self), fields(name = self.book_name()) )]
    async fn save(&mut self) {
        self.update_white_list_with_conn_list();
        tracing::trace!(
            "white_len: {}, gray_len: {}, anchor_len: {}, banned_len: {}",
            self.len_white_list(),
            self.len_gray_list(),
            self.len_anchor_list(),
            self.len_banned_list()
        );
        let res = self
            .p2p_store
            .save_peers(
                self.zone,
                (&self.white_list).into(),
                (&self.gray_list).into(),
                self.anchor_list.iter().collect(),
            )
            .await;
        match res {
            Ok(()) => tracing::trace!("Complete"),
            Err(e) => tracing::error!("Error saving address book: {e}"),
        }
    }

    /// Runs the address book task
    /// Should be spawned in a task.
    pub(crate) async fn run(mut self, mut rx: mpsc::Receiver<AddressBookClientRequest>) {
        let mut save_interval = {
            let mut interval = tokio::time::interval(ADDRESS_BOOK_SAVE_INTERVAL);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            // Interval ticks at 0, interval, 2 interval, ...
            // this is just to ignore the first tick
            interval.tick().await;
            tokio_stream::wrappers::IntervalStream::new(interval).fuse()
        };

        loop {
            self.check_unban_peers();
            self.check_connected_peers();
            futures::select! {
                req = rx.next() => {
                    if let Some(req) = req {
                        self.handle_request(req).await
                    } else {
                        tracing::debug!("{} req channel closed, saving and shutting down book", self.book_name());
                        self.save().await;
                        return;
                    }
                }
                _ = save_interval.next() => self.save().await
            }
        }
    }
}

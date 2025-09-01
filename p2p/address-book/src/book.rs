//! The address book service.
//!
//! This module holds the address book service for a specific network zone.
use std::{
    collections::{HashMap, HashSet},
    panic,
    task::{Context, Poll},
    time::Duration,
};

use futures::{
    future::{ready, Ready},
    FutureExt,
};
use tokio::{
    task::JoinHandle,
    time::{interval, Instant, Interval, MissedTickBehavior},
};
use tokio_util::time::DelayQueue;
use tower::Service;

use cuprate_p2p_core::{
    client::InternalPeerID,
    handles::ConnectionHandle,
    services::{AddressBookRequest, AddressBookResponse, ZoneSpecificPeerListEntryBase},
    NetZoneAddress, NetworkZone,
};
use cuprate_pruning::PruningSeed;

use crate::anchors::{AnchorList, AnchorPeer};
use crate::{
    peer_list::PeerList, store::save_peers_to_disk, AddressBookConfig, AddressBookError, BanList,
    BorshNetworkZone,
};

#[cfg(test)]
mod tests;

/// An entry in the connected list.
pub(crate) struct ConnectionPeerEntry<Z: NetworkZone> {
    addr: Option<Z::Addr>,
    id: u64,
    handle: ConnectionHandle,
    /// The peers pruning seed
    pruning_seed: PruningSeed,
    /// The peers port.
    rpc_port: u16,
    /// The peers rpc credits per hash
    rpc_credits_per_hash: u32,
}

pub struct AddressBook<Z: BorshNetworkZone, B: BanList<Z::Addr>> {
    /// Our white peers - the peers we have previously connected to.
    white_list: PeerList<Z>,
    /// Our gray peers - the peers we have been told about but haven't connected to.
    gray_list: PeerList<Z>,
    /// Our anchor peers - on start up will contain a list of peers we were connected to before shutting down
    /// after that will contain a list of peers currently connected to that we can reach.
    anchor_list: AnchorList<Z>,
    /// The currently connected peers.
    connected_peers: HashMap<InternalPeerID<Z::Addr>, ConnectionPeerEntry<Z>>,
    connected_peers_ban_id: HashMap<<Z::Addr as NetZoneAddress>::BanID, HashSet<Z::Addr>>,

    ban_list: B,

    peer_save_task_handle: Option<JoinHandle<std::io::Result<()>>>,
    peer_save_interval: Interval,

    cfg: AddressBookConfig<Z>,
}

impl<Z: BorshNetworkZone, B: BanList<Z::Addr>> AddressBook<Z, B> {
    pub fn new(
        cfg: AddressBookConfig<Z>,
        white_peers: Vec<ZoneSpecificPeerListEntryBase<Z::Addr>>,
        gray_peers: Vec<ZoneSpecificPeerListEntryBase<Z::Addr>>,
        anchors: Vec<AnchorPeer<Z::Addr>>,
        ban_list: B,
    ) -> Self {
        let white_list = PeerList::new(white_peers);
        let gray_list = PeerList::new(gray_peers);
        let anchor_list = AnchorList::new(anchors);

        let mut peer_save_interval = interval(cfg.peer_save_period);
        peer_save_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        Self {
            white_list,
            gray_list,
            anchor_list,
            connected_peers: HashMap::new(),
            connected_peers_ban_id: HashMap::new(),
            ban_list,
            peer_save_task_handle: None,
            peer_save_interval,
            cfg,
        }
    }

    fn poll_save_to_disk(&mut self, cx: &mut Context<'_>) {
        if let Some(handle) = &mut self.peer_save_task_handle {
            // if we have already spawned a task to save the peer list wait for that to complete.
            match handle.poll_unpin(cx) {
                Poll::Pending => return,
                Poll::Ready(Ok(Err(e))) => {
                    tracing::error!("Could not save peer list to disk, got error: {e}");
                }
                Poll::Ready(Err(e)) => {
                    if e.is_panic() {
                        panic::resume_unwind(e.into_panic())
                    }
                }
                Poll::Ready(_) => (),
            }
        }
        // the task is finished.
        self.peer_save_task_handle = None;

        let Poll::Ready(_) = self.peer_save_interval.poll_tick(cx) else {
            return;
        };

        self.peer_save_task_handle = Some(save_peers_to_disk(
            &self.cfg,
            &self.white_list,
            &self.gray_list,
            &self.anchor_list,
            &self.ban_list,
        ));
    }

    fn poll_connected_peers(&mut self) {
        let mut internal_addr_disconnected = Vec::new();
        let mut addrs_to_ban = Vec::new();

        #[expect(clippy::iter_over_hash_type, reason = "ordering doesn't matter here")]
        for (internal_addr, peer) in &mut self.connected_peers {
            if let Some(time) = peer.handle.check_should_ban() {
                match internal_addr {
                    InternalPeerID::KnownAddr(addr) => addrs_to_ban.push((addr.clone(), time.0)),
                    // If we don't know the peers address all we can do is disconnect.
                    InternalPeerID::Unknown(_) => peer.handle.send_close_signal(),
                }
            }

            if peer.handle.is_closed() {
                internal_addr_disconnected.push(internal_addr.clone());
            }
        }

        for (addr, time) in addrs_to_ban {
            self.ban_peer(addr, time);
        }

        for disconnected_addr in internal_addr_disconnected {
            self.connected_peers.remove(&disconnected_addr);
            if let InternalPeerID::KnownAddr(addr) = disconnected_addr {
                // remove the peer from the connected peers with this ban ID.
                self.connected_peers_ban_id
                    .get_mut(&addr.ban_id())
                    .unwrap()
                    .remove(&addr);

                // If the amount of peers with this ban id is 0 remove the whole set.
                if self.connected_peers_ban_id[&addr.ban_id()].is_empty() {
                    self.connected_peers_ban_id.remove(&addr.ban_id());
                }
            }
        }
    }

    fn ban_peer(&mut self, addr: Z::Addr, time: Duration) {
        if let Some(connected_peers_with_ban_id) = self.connected_peers_ban_id.get(&addr.ban_id()) {
            for peer in connected_peers_with_ban_id.iter().map(|addr| {
                tracing::debug!("Banning peer: {}, for: {:?}", addr, time);

                self.connected_peers
                    .get(&InternalPeerID::KnownAddr(addr.clone()))
                    .expect("Peer must be in connected list if in connected_peers_with_ban_id")
            }) {
                // The peer will get removed from our connected list once we disconnect
                peer.handle.send_close_signal();
            }
        }

        self.anchor_list.remove(&addr);
        self.white_list.remove_peers_with_ban_id(&addr.ban_id());
        self.gray_list.remove_peers_with_ban_id(&addr.ban_id());

        self.ban_list.ban(addr.ban_id(), time);
    }

    /// adds a peer to the gray list.
    fn add_peer_to_gray_list(&mut self, mut peer: ZoneSpecificPeerListEntryBase<Z::Addr>) {
        if self.gray_list.len() >= self.cfg.max_gray_list_length {
            return;
        }

        if self.anchor_list.contains(&peer.adr) {
            tracing::trace!("Peer {} is already in anchor list skipping.", peer.adr);
            return;
        }
        if self.white_list.contains_peer(&peer.adr) {
            tracing::trace!("Peer {} is already in white list skipping.", peer.adr);
            return;
        }
        if !self.gray_list.contains_peer(&peer.adr) {
            tracing::trace!("Adding peer {} to gray list.", peer.adr);
            peer.last_seen = 0;
            self.gray_list.add_new_peer(peer);
        }
    }

    /// adds a peer to the gray list.
    fn add_peer_to_white_list(&mut self, mut peer: ZoneSpecificPeerListEntryBase<Z::Addr>) {
        if self.white_list.len() >= self.cfg.max_white_list_length {
            return;
        }

        if self.anchor_list.contains(&peer.adr) {
            tracing::trace!("Peer {} is already in anchor list skipping.", peer.adr);
            return;
        }
        if self.gray_list.contains_peer(&peer.adr) {
            tracing::trace!("Peer {} is already in grey list skipping.", peer.adr);
            return;
        }
        if !self.white_list.contains_peer(&peer.adr) {
            tracing::trace!("Adding peer {} to white list.", peer.adr);
            peer.last_seen = 0;
            self.white_list.add_new_peer(peer);
        }
    }

    /// Checks when a peer will be unbanned.
    ///
    /// - If the peer is banned, this returns [`Some`] containing
    ///   the [`Instant`] the peer will be unbanned
    /// - If the peer is not banned, this returns [`None`]
    fn peer_unban_instant(&self, peer: &Z::Addr) -> Option<Instant> {
        self.ban_list.unbanned_instant(&peer)
    }

    fn handle_incoming_peer_list(
        &mut self,
        mut peer_list: Vec<ZoneSpecificPeerListEntryBase<Z::Addr>>,
    ) {
        tracing::debug!("Received new peer list, length: {}", peer_list.len());

        peer_list.retain_mut(|peer| {
            peer.adr.make_canonical();

            peer.adr.should_add_to_peer_list() && !self.ban_list.is_banned(&peer.adr)
            // TODO: check rpc/ p2p ports not the same
        });

        for peer in peer_list {
            self.add_peer_to_gray_list(peer);
        }
    }

    fn take_random_white_peer(
        &mut self,
        block_needed: Option<usize>,
    ) -> Option<ZoneSpecificPeerListEntryBase<Z::Addr>> {
        tracing::debug!("Retrieving random white peer");
        self.white_list.take_random_peer(
            &self
                .connected_peers
                .keys()
                .filter_map(|k| match k {
                    InternalPeerID::KnownAddr(addr) => Some(addr.clone()),
                    InternalPeerID::Unknown(_) => None,
                })
                .collect::<Vec<_>>(),
            &mut rand::thread_rng(),
        )
    }

    fn take_random_gray_peer(
        &mut self,
        block_needed: Option<usize>,
    ) -> Option<ZoneSpecificPeerListEntryBase<Z::Addr>> {
        tracing::debug!("Retrieving random gray peer");
        self.gray_list
            .take_random_peer(&[], &mut rand::thread_rng())
    }

    pub fn get_anchor_peers(
        &mut self,
        include_connected: bool,
        len: usize,
    ) -> Vec<ZoneSpecificPeerListEntryBase<Z::Addr>> {
        while len > self.anchor_list.len() {
            let Some(peer) = self
                .take_random_white_peer(None)
                .or_else(|| self.take_random_gray_peer(None))
            else {
                break;
            };
            self.anchor_list.add(peer);
        }

        self.anchor_list
            .anchors()
            .values()
            .filter(|a| {
                include_connected
                    || !self
                        .connected_peers
                        .contains_key(&InternalPeerID::KnownAddr(a.peer.adr.clone()))
            })
            .map(|a| a.peer.clone())
            .collect()
    }

    fn get_white_peers(&self, len: usize) -> Vec<ZoneSpecificPeerListEntryBase<Z::Addr>> {
        tracing::debug!("Retrieving white peers, maximum: {}", len);
        self.white_list
            .get_random_peers(&mut rand::thread_rng(), len)
    }

    /// Updates an entry in the white list, if the peer is not found then
    /// the peer will be added to the white list.
    fn update_white_list_peer_entry(
        &mut self,
        peer: &ConnectionPeerEntry<Z>,
    ) -> Result<(), AddressBookError> {
        let Some(addr) = &peer.addr else {
            // If the peer isn't reachable we shouldn't add it too our address book.
            return Ok(());
        };

        if self.anchor_list.contains(addr) {
            return Ok(());
        }

        if let Some(peb) = self.white_list.get_peer_mut(addr) {
            if peb.pruning_seed != peer.pruning_seed {
                return Err(AddressBookError::PeersDataChanged("Pruning seed"));
            }
            // TODO: cuprate doesn't need last seen timestamps but should we have them anyway?
            peb.last_seen = 0;
            peb.rpc_port = peer.rpc_port;
            peb.rpc_credits_per_hash = peer.rpc_credits_per_hash;
        } else {
            if self.white_list.len() >= self.cfg.max_white_list_length {
                return Ok(());
            }

            // if the peer is reachable add it to our white list
            let peb = ZoneSpecificPeerListEntryBase {
                id: peer.id,
                adr: addr.clone(),
                last_seen: 0,
                rpc_port: peer.rpc_port,
                rpc_credits_per_hash: peer.rpc_credits_per_hash,
                pruning_seed: peer.pruning_seed,
            };
            self.white_list.add_new_peer(peb);
        }
        Ok(())
    }

    fn handle_new_connection(
        &mut self,
        internal_peer_id: InternalPeerID<Z::Addr>,
        peer: ConnectionPeerEntry<Z>,
    ) -> Result<(), AddressBookError> {
        if self.connected_peers.contains_key(&internal_peer_id) {
            return Err(AddressBookError::PeerAlreadyConnected);
        }

        // If we know the address then check if it's banned.
        if let InternalPeerID::KnownAddr(addr) = &internal_peer_id {
            if self.ban_list.is_banned(addr) {
                return Err(AddressBookError::PeerIsBanned);
            }
            // although the peer may not be reachable still add it to the connected peers with ban ID.
            self.connected_peers_ban_id
                .entry(addr.ban_id())
                .or_default()
                .insert(addr.clone());
        }

        self.update_white_list_peer_entry(&peer)?;

        self.connected_peers.insert(internal_peer_id, peer);
        Ok(())
    }

    fn all_peers(&self) -> cuprate_p2p_core::types::Peerlist<Z::Addr> {
        cuprate_p2p_core::types::Peerlist {
            anchors: self
                .anchor_list
                .anchors()
                .values()
                .map(|a| a.peer.clone())
                .collect(),
            white: self.white_list.peers.values().cloned().collect(),
            grey: self.gray_list.peers.values().cloned().collect(),
        }
    }
}

impl<Z: BorshNetworkZone, B: BanList<Z::Addr>> Service<AddressBookRequest<Z>>
    for AddressBook<Z, B>
{
    type Response = AddressBookResponse<Z>;
    type Error = AddressBookError;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.ban_list.poll_bans(cx);
        self.poll_save_to_disk(cx);
        self.anchor_list.poll_timeouts(cx);
        self.poll_connected_peers();
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: AddressBookRequest<Z>) -> Self::Future {
        let span = tracing::info_span!("AddressBook");
        let _guard = span.enter();

        let response = match req {
            AddressBookRequest::NewConnection {
                internal_peer_id,
                public_address,
                handle,
                id,
                pruning_seed,
                rpc_port,
                rpc_credits_per_hash,
            } => self
                .handle_new_connection(
                    internal_peer_id,
                    ConnectionPeerEntry {
                        addr: public_address,
                        id,
                        handle,
                        pruning_seed,
                        rpc_port,
                        rpc_credits_per_hash,
                    },
                )
                .map(|()| AddressBookResponse::Ok),
            AddressBookRequest::PeerReachable(peer) => {
                self.add_peer_to_white_list(peer);
                Ok(AddressBookResponse::Ok)
            }
            AddressBookRequest::IncomingPeerList(_, peer_list) => {
                self.handle_incoming_peer_list(peer_list);
                Ok(AddressBookResponse::Ok)
            }
            AddressBookRequest::TakeRandomWhitePeer { height } => self
                .take_random_white_peer(height)
                .map(AddressBookResponse::Peer)
                .ok_or(AddressBookError::PeerNotFound),
            AddressBookRequest::TakeRandomGrayPeer { height } => self
                .take_random_gray_peer(height)
                .map(AddressBookResponse::Peer)
                .ok_or(AddressBookError::PeerNotFound),
            AddressBookRequest::TakeRandomPeer { height } => self
                .take_random_white_peer(height)
                .or_else(|| self.take_random_gray_peer(height))
                .map(AddressBookResponse::Peer)
                .ok_or(AddressBookError::PeerNotFound),
            AddressBookRequest::GetAnchorPeers {
                include_connected,
                len,
            } => Ok(AddressBookResponse::Peers(
                self.get_anchor_peers(include_connected, len),
            )),
            AddressBookRequest::RemoveAnchorPeer(addr) => {
                self.anchor_list.remove(&addr);
                Ok(AddressBookResponse::Ok)
            }
            AddressBookRequest::GetWhitePeers(len) => {
                Ok(AddressBookResponse::Peers(self.get_white_peers(len)))
            }
            AddressBookRequest::GetBan(addr) => Ok(AddressBookResponse::GetBan {
                unban_instant: self.peer_unban_instant(&addr).map(Instant::into_std),
            }),
            AddressBookRequest::OwnAddress => Ok(AddressBookResponse::OwnAddress(
                self.cfg.our_own_address.clone(),
            )),
            AddressBookRequest::Peerlist => Ok(AddressBookResponse::Peerlist(self.all_peers())),
            AddressBookRequest::PeerlistSize
            | AddressBookRequest::ConnectionCount
            | AddressBookRequest::SetBan(_)
            | AddressBookRequest::GetBans
            | AddressBookRequest::ConnectionInfo => {
                todo!("finish https://github.com/Cuprate/cuprate/pull/297")
            }
        };

        ready(response)
    }
}

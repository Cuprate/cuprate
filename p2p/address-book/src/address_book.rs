mod peer_list;

use std::collections::{HashSet, HashMap};

use cuprate_common::PruningSeed;
use monero_wire::{NetworkAddress, messages::PeerListEntryBase, network_address::NetZone};
use futures::{channel::{mpsc, oneshot}, StreamExt};
use rand::Rng;

use peer_list::PeerList;
use super::{AddressBookError, MAX_GRAY_LIST_PEERS, MAX_WHITE_LIST_PEERS};





pub enum AddressBookRequest {
    HandleNewPeerList(Vec<PeerListEntryBase>, NetZone),
    SetPeerSeen(NetworkAddress, i64),
    BanPeer(NetworkAddress, chrono::NaiveDateTime),
    AddPeerToAnchor(NetworkAddress),
    RemovePeerFromAnchor(NetworkAddress),

    GetRandomGrayPeer(NetZone),
}

impl AddressBookRequest {
    pub fn get_zone(&self) -> NetZone {
        match self {
            Self::HandleNewPeerList(_, zone) => *zone,
            Self::SetPeerSeen(peer, _) => peer.get_zone(),
            Self::BanPeer(peer, _) => peer.get_zone(),
            Self::AddPeerToAnchor(peer) => peer.get_zone(),
            Self::RemovePeerFromAnchor(peer) => peer.get_zone(),
            Self::GetRandomGrayPeer(zone) => *zone,

        }
    }
}

pub enum AddressBookResponse {
    Ok,
    Peer(PeerListEntryBase),
}

pub struct AddressBookClientRequest {
    pub req: AddressBookRequest,
    pub tx: oneshot::Sender<Result<AddressBookResponse, AddressBookError>>,
}

pub struct AddressBookConfig {
    max_white_peers: usize,
    max_gray_peers:  usize,
}

impl Default for AddressBookConfig {
    fn default() -> Self {
        AddressBookConfig { 
            max_white_peers: MAX_WHITE_LIST_PEERS, 
            max_gray_peers: MAX_GRAY_LIST_PEERS 
        }
    }
}



pub struct AddressBook<R> {
    zone: NetZone,
    config: AddressBookConfig,
    white_list: PeerList,
    gray_list: PeerList,
    anchor_list: HashSet<NetworkAddress>,

    baned_peers: HashMap<NetworkAddress, chrono::NaiveDateTime>,

    rng: R,
   //banned_subnets:,
}

impl<R: Rng> AddressBook<R> {
    pub fn new() {
        todo!()
    }

    fn len_white_list(&self) -> usize {
        self.white_list.len()
    }

    fn len_gray_list(&self) -> usize {
        self.gray_list.len()
    }

    fn max_white_peers(&self) -> usize {
        self.config.max_white_peers
    }

    fn max_gray_peers(&self) -> usize {
        self.config.max_gray_peers
    }

    fn is_peer_banned(&self, peer: &NetworkAddress) -> bool {
        self.baned_peers.contains_key(peer)
    }

    fn check_unban_peers(&mut self) {
        let mut now = chrono::Utc::now().naive_utc();
        self.baned_peers.retain(|_, time|
            time > &mut now
        )
    }

    fn ban_peer(&mut self, peer: NetworkAddress, till: chrono::NaiveDateTime) {
        let now = chrono::Utc::now().naive_utc();
        if now > till {
            return;
        }

        self.baned_peers.insert(peer, till);
    }

    fn add_peer_to_anchor(&mut self, peer: NetworkAddress) -> Result<(), AddressBookError> {
       // is peer in gray list
       if let Some(peer_eb) = self.gray_list.remove_peer(&peer) {
            self.white_list.add_new_peer(peer_eb);
            self.anchor_list.insert(peer);
            Ok(())
        } else {
            if !self.white_list.contains_peer(&peer) {
                return Err(AddressBookError::PeerNotFound);
            }
            self.anchor_list.insert(peer);
            Ok(())
        }
    }

    fn remove_peer_from_anchor(&mut self, peer: NetworkAddress) {
        let _ = self.anchor_list.remove(&peer);
    }

    fn set_peer_seen(&mut self, peer: NetworkAddress, last_seen: i64) -> Result<(), AddressBookError>{
        // is peer in gray list
        if let Some(mut peer) = self.gray_list.remove_peer(&peer) {
            peer.last_seen = last_seen;
            self.white_list.add_new_peer(peer);
        } else {
            let peer = self.white_list.get_peer_mut(&peer).ok_or(AddressBookError::PeerNotFound)?;
            peer.last_seen = last_seen;
        }
        Ok(())
    } 

    fn add_peer_to_gray_list(&mut self, mut peer: PeerListEntryBase) {
        if self.white_list.contains_peer(&peer.adr) {
            return;
        };
        if !self.gray_list.contains_peer(&peer.adr) {
            peer.last_seen = 0;
            self.gray_list.add_new_peer(peer);
        }
    }

    fn handle_new_peerlist(&mut self, mut peers: Vec<PeerListEntryBase>) -> Result<(), AddressBookError> {
        let mut err = None;
        peers.retain(|peer|
            if err.is_some() {
                false
            } else if peer.adr.is_local() || peer.adr.is_loopback() {
                false
            } else if peer.adr.port() == peer.rpc_port {
                false
            } else if PruningSeed::try_from(peer.pruning_seed).is_err() {
                false
            } else if peer.adr.get_zone() != self.zone {
                err = Some(AddressBookError::PeerSentAnAddressOutOfZone);
                false
            } else if self.is_peer_banned(&peer.adr) {
                false
            } else {
                true
            }
        );

        if let Some(e) = err {
            return Err(e);
        } else {
            for peer in peers {
                self.add_peer_to_gray_list(peer);
            }
            self.gray_list.reduce_list(&HashSet::new(), self.max_gray_peers());
            Ok(())
        }
    } 

    pub fn get_random_gray_peer(&mut self) -> Option<PeerListEntryBase> {
        let gray_len = self.len_gray_list();
        if gray_len == 0 {
            None
        } else {
            let n = self.rng.gen_range(0..gray_len);

            self.gray_list.get_peer_by_idx(n).map(|p| *p)
        }
    }

    pub async fn run(mut self, mut rx: mpsc::Receiver<AddressBookClientRequest>) {
        loop {
            let Some(req) = rx.next().await else {
                // the client has been dropped the node has *possibly* shut down 
                return;
            };

            self.check_unban_peers();


            let res = match req.req {
                AddressBookRequest::HandleNewPeerList(new_peers, _) => self.handle_new_peerlist(new_peers).map(|_| AddressBookResponse::Ok),
                AddressBookRequest::SetPeerSeen(peer, last_seen) => self.set_peer_seen(peer, last_seen).map(|_| AddressBookResponse::Ok),
                AddressBookRequest::BanPeer(peer, till) => {self.ban_peer(peer, till); Ok(AddressBookResponse::Ok)},
                AddressBookRequest::AddPeerToAnchor(peer) => self.add_peer_to_anchor(peer).map(|_| AddressBookResponse::Ok),
                AddressBookRequest::RemovePeerFromAnchor(peer) => {self.remove_peer_from_anchor(peer); Ok(AddressBookResponse::Ok)},

                AddressBookRequest::GetRandomGrayPeer(_) =>  {
                    match self.get_random_gray_peer() {
                        Some(peer) => Ok(AddressBookResponse::Peer(peer)),
                        None => Err(AddressBookError::PeerNotFound),
                    }
                },

            };

            let _ = req.tx.send(res);
        }
    }

    
}
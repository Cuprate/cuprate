
use std::collections::{HashSet, HashMap};

use std::time::Duration;
use futures::{channel::{mpsc, oneshot}, StreamExt};
use rand::{Rng, SeedableRng};

use cuprate_common::PruningSeed;
use monero_wire::{NetworkAddress, messages::PeerListEntryBase, network_address::NetZone};

use super::{AddressBookError, AddressBookRequest, AddressBookResponse, AddressBookConfig};

mod peer_list;
use peer_list::PeerList;






pub(crate) struct AddressBookClientRequest {
    pub req: AddressBookRequest,
    pub tx: oneshot::Sender<Result<AddressBookResponse, AddressBookError>>,

    pub span: tracing::Span,
}


pub struct AddressBook {
    zone: NetZone,
    config: AddressBookConfig,
    white_list: PeerList,
    gray_list: PeerList,
    anchor_list: HashSet<NetworkAddress>,

    baned_peers: HashMap<NetworkAddress, chrono::NaiveDateTime>,

    rng: rand::rngs::StdRng,
   //banned_subnets:,
}

impl AddressBook {
    pub fn new(
        config:AddressBookConfig, 
        zone: NetZone, 
        white_peers: Vec<PeerListEntryBase>,
        gray_peers: Vec<PeerListEntryBase>,
        anchor_peers: Vec<NetworkAddress>,
        baned_peers: Vec<(NetworkAddress, chrono::NaiveDateTime)>,
    ) -> AddressBook {
        let rng = rand::prelude::StdRng::from_entropy();
        let white_list = PeerList::new(white_peers);
        let gray_list = PeerList::new(gray_peers);
        let anchor_list = HashSet::from_iter(anchor_peers);
        let baned_peers = HashMap::from_iter(baned_peers);

        let mut book = AddressBook{
            zone,
            config,
            white_list,
            gray_list,
            anchor_list,
            baned_peers,
            rng
        };

        book.check_unban_peers();

        book

        
    }

    pub const fn book_name(&self) -> &'static str {
        match self.zone {
            NetZone::Public => "PublicAddressBook",
            NetZone::Tor => "TorAddressBook",
            NetZone::I2p => "I2pAddressBook",

        }
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

        tracing::debug!("Banning peer: {peer:?} until: {till}");

        self.baned_peers.insert(peer, till);
    }

    fn add_peer_to_anchor(&mut self, peer: NetworkAddress) -> Result<(), AddressBookError> {

        tracing::debug!("Adding peer: {peer:?} to anchor list");
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
        let length = peers.len();

        tracing::debug!("Received new peer list, length: {length}");

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
                tracing::info!("Received an address from a different network zone, ignoring list.");
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

    fn get_random_gray_peer(&mut self) -> Option<PeerListEntryBase> {
        self.gray_list.get_random_peer(&mut self.rng).map(|p| *p)
    }

    fn get_random_white_peer(&mut self) -> Option<PeerListEntryBase> {
        self.white_list.get_random_peer(&mut self.rng).map(|p| *p)
    }

    fn update_peer_info(&mut self, peer: PeerListEntryBase) -> Result<(), AddressBookError> {
        if let Some(peer_stored) = self.gray_list.get_peer_mut(&peer.adr) {
            *peer_stored = peer;
            Ok(())
        } else if let Some(peer_stored) = self.white_list.get_peer_mut(&peer.adr) {
            *peer_stored = peer;
            Ok(())
        } else {
            return Err(AddressBookError::PeerNotFound);
        }
    }

    pub(crate) async fn run(mut self, mut rx: mpsc::Receiver<AddressBookClientRequest>) {
        loop {
            let Some(req) = rx.next().await else {
                // the client has been dropped the node has *possibly* shut down 
                return;
            };

            self.check_unban_peers();

            let span = tracing::debug_span!(parent: &req.span,  "AddressBook");
            let _guard = span.enter();

            tracing::debug!("{} received request: {}", self.book_name(), req.req);



            let res = match req.req {
                AddressBookRequest::HandleNewPeerList(new_peers, _) => self.handle_new_peerlist(new_peers).map(|_| AddressBookResponse::Ok),
                AddressBookRequest::SetPeerSeen(peer, last_seen) => self.set_peer_seen(peer, last_seen).map(|_| AddressBookResponse::Ok),
                AddressBookRequest::BanPeer(peer, till) => {self.ban_peer(peer, till); Ok(AddressBookResponse::Ok)},
                AddressBookRequest::AddPeerToAnchor(peer) => self.add_peer_to_anchor(peer).map(|_| AddressBookResponse::Ok),
                AddressBookRequest::RemovePeerFromAnchor(peer) => {self.remove_peer_from_anchor(peer); Ok(AddressBookResponse::Ok)},
                AddressBookRequest::UpdatePeerInfo(peer) => self.update_peer_info(peer).map(|_| AddressBookResponse::Ok),

                AddressBookRequest::GetRandomGrayPeer(_) =>  {
                    match self.get_random_gray_peer() {
                        Some(peer) => Ok(AddressBookResponse::Peer(peer)),
                        None => Err(AddressBookError::PeerListEmpty),
                    }
                },
                AddressBookRequest::GetRandomWhitePeer(_) => {
                    match self.get_random_white_peer() {
                        Some(peer) => Ok(AddressBookResponse::Peer(peer)),
                        None => Err(AddressBookError::PeerListEmpty),
                    }
                }

            };

            if let Err(e) = &res {
                tracing::debug!("Error when handling request, err: {e}")
            }

            let _ = req.tx.send(res);
        }
    }

    
}
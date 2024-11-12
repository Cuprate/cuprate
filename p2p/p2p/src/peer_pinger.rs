//! Peer Pinger.
//! 
//! This module handles the connectivity check to the peers contain in our address book.
//! It make sure to periodically remove any peers that are unreachable.

use std::{marker::PhantomData, time::Duration};

use cuprate_p2p_core::{client::handshaker::ping, services::{AddressBookRequest, AddressBookResponse}, AddressBook, NetworkZone};
use tokio::time::{sleep, timeout};
use tracing::warn;

use crate::constants::PING_REQUEST_TIMEOUT;

pub(crate) struct PeerPinger<N: NetworkZone, A: AddressBook<N>> {
    address_book_svc: A,
    delay_btw_ping: Duration,
    _network: PhantomData<N>
}

impl<
    N: NetworkZone,
    A: AddressBook<N>
> PeerPinger<N,A> {
    pub(crate) const fn new(address_book_svc: A, delay_btw_ping: Duration) -> Self {
        Self {
            address_book_svc,
            delay_btw_ping,
            _network: PhantomData,
        }
    }
    
    /// Will ping a random white and gray peer every `self.delay_btw_ping`.
    /// Only replace the peer back in the list if it has been reached.
    pub(crate) async fn run(mut self) {
        
        loop {
            sleep(self.delay_btw_ping).await;
            
            // First ping a white peer
            let Ok(AddressBookResponse::Peer(peer)) = self.address_book_svc.call(AddressBookRequest::TakeRandomWhitePeer { height: None }).await else {
                warn!("AddressBook unavailable.");
                return
            };
            
            let response = timeout(PING_REQUEST_TIMEOUT, ping::<N>(peer.adr)).await;
            if let Ok(Ok(peer_id)) = response {
                if peer_id == peer.id {
                    let Ok(AddressBookResponse::Ok) = self.address_book_svc.call(AddressBookRequest::AddWhitePeer(peer)).await else {
                        warn!("AddressBook unavailable.");
                        return
                    };
                }
            }
            
            // Then ping a gray peer
            let Ok(AddressBookResponse::Peer(peer)) = self.address_book_svc.call(AddressBookRequest::TakeRandomGrayPeer { height: None }).await else {
                warn!("AddressBook unavailable.");
                return
            };
            
            let response = timeout(PING_REQUEST_TIMEOUT, ping::<N>(peer.adr)).await;
            if let Ok(Ok(peer_id)) = response {
                if peer_id == peer.id {
                    let Ok(AddressBookResponse::Ok) = self.address_book_svc.call(AddressBookRequest::IncomingPeerList(vec![peer])).await else {
                        warn!("AddressBook unavailable.");
                        return
                    };
                }
            }
        }
    }
}

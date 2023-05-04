mod addr_book_client;
pub(crate) mod address_book;

pub use addr_book_client::start_address_book;

use monero_wire::{messages::PeerListEntryBase, network_address::NetZone, NetworkAddress};

use crate::constants::{MAX_GRAY_LIST_PEERS, MAX_WHITE_LIST_PEERS};

#[derive(Debug, thiserror::Error)]
pub enum AddressBookError {
    #[error("Peer was not found in book")]
    PeerNotFound,
    #[error("The peer list is empty")]
    PeerListEmpty,
    #[error("Peer sent an address out of it's net-zone")]
    PeerSentAnAddressOutOfZone,
    #[error("The address books channel has closed.")]
    AddressBooksChannelClosed,
    #[error("Peer Store Error: {0}")]
    PeerStoreError(&'static str),
}

#[derive(Debug)]
pub enum AddressBookRequest {
    HandleNewPeerList(Vec<PeerListEntryBase>, NetZone),
    SetPeerSeen(NetworkAddress, i64),
    BanPeer(NetworkAddress, chrono::NaiveDateTime),
    AddPeerToAnchor(NetworkAddress),
    RemovePeerFromAnchor(NetworkAddress),
    UpdatePeerInfo(PeerListEntryBase),

    GetRandomGrayPeer(NetZone),
    GetRandomWhitePeer(NetZone),
}

impl std::fmt::Display for AddressBookRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HandleNewPeerList(_, _) => f.write_str("HandleNewPeerList"),
            Self::SetPeerSeen(_, _) => f.write_str("SetPeerSeen"),
            Self::BanPeer(_, _) => f.write_str("BanPeer"),
            Self::AddPeerToAnchor(_) => f.write_str("AddPeerToAnchor"),
            Self::RemovePeerFromAnchor(_) => f.write_str("RemovePeerFromAnchor"),
            Self::UpdatePeerInfo(_) => f.write_str("UpdatePeerInfo"),
            Self::GetRandomGrayPeer(_) => f.write_str("GetRandomGrayPeer"),
            Self::GetRandomWhitePeer(_) => f.write_str("GetRandomWhitePeer"),
        }
    }
}

impl AddressBookRequest {
    pub fn get_zone(&self) -> NetZone {
        match self {
            Self::HandleNewPeerList(_, zone) => *zone,
            Self::SetPeerSeen(peer, _) => peer.get_zone(),
            Self::BanPeer(peer, _) => peer.get_zone(),
            Self::AddPeerToAnchor(peer) => peer.get_zone(),
            Self::RemovePeerFromAnchor(peer) => peer.get_zone(),
            Self::UpdatePeerInfo(peer) => peer.adr.get_zone(),

            Self::GetRandomGrayPeer(zone) => *zone,
            Self::GetRandomWhitePeer(zone) => *zone,
        }
    }
}

#[derive(Debug)]
pub enum AddressBookResponse {
    Ok,
    Peer(PeerListEntryBase),
}

#[async_trait::async_trait]
pub trait AddressBookStore: Clone {
    type Error: Into<AddressBookError>;
    /// Loads the peers from the peer store.
    /// returns (in order):
    /// the white list,
    /// the gray list,
    /// the anchor list,
    /// the ban list
    async fn load_peers(
        &mut self,
        zone: NetZone,
    ) -> Result<
        (
            Vec<PeerListEntryBase>,                       // white list
            Vec<PeerListEntryBase>,                       // gray list
            Vec<NetworkAddress>,                          // anchor list
            Vec<(NetworkAddress, chrono::NaiveDateTime)>, // ban list
        ),
        Self::Error,
    >;

    async fn save_peers(
        &mut self,
        zone: NetZone,
        white: Vec<PeerListEntryBase>,
        gray: Vec<PeerListEntryBase>,
        anchor: Vec<NetworkAddress>,
        bans: Vec<(NetworkAddress, chrono::NaiveDateTime)>, // ban lists
    ) -> Result<(), Self::Error>;
}

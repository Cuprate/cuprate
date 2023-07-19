//! Cuprate Address Book
//!
//! This module holds the logic for persistent peer storage.
//! Cuprates address book is modeled as a [`tower::Service`]
//! The request is [`AddressBookRequest`] and the response is
//! [`AddressBookResponse`].
//!
//! Cuprate, like monerod, actually has 3 address books, one
//! for each [`NetZone`]. This is to reduce the possibility of
//! clear net peers getting linked to their dark counterparts
//! and so peers will only get told about peers they can
//! connect to.
//!

mod addr_book_client;
mod address_book;
pub mod connection_handle;

use cuprate_common::PruningSeed;
use monero_wire::{messages::PeerListEntryBase, network_address::NetZone, NetworkAddress, PeerID};

use connection_handle::ConnectionAddressBookHandle;

pub use addr_book_client::start_address_book;

/// Possible errors when dealing with the address book.
/// This is boxed when returning an error in the [`tower::Service`].
#[derive(Debug, thiserror::Error)]
pub enum AddressBookError {
    /// The peer is not in the address book for this zone.
    #[error("Peer was not found in book")]
    PeerNotFound,
    /// The peer list is empty.
    #[error("The peer list is empty")]
    PeerListEmpty,
    /// The peers pruning seed has changed.
    #[error("The peers pruning seed has changed")]
    PeersPruningSeedChanged,
    /// The peer is banned.
    #[error("The peer is banned")]
    PeerIsBanned,
    /// When handling a received peer list, the list contains
    /// a peer in a different [`NetZone`]
    #[error("Peer sent an address out of it's net-zone")]
    PeerSentAnAddressOutOfZone,
    /// The channel to the address book has closed unexpectedly.
    #[error("The address books channel has closed.")]
    AddressBooksChannelClosed,
    /// The address book task has exited.
    #[error("The address book task has exited.")]
    AddressBookTaskExited,
    /// The peer file store has failed.
    #[error("Peer Store Error: {0}")]
    PeerStoreError(&'static str),
}

/// A message sent to tell the address book that a peer has disconnected.
pub struct PeerConnectionClosed;

/// A request to the address book.
#[derive(Debug)]
pub enum AddressBookRequest {
    /// A request to handle an incoming peer list.
    HandleNewPeerList(Vec<PeerListEntryBase>, NetZone),
    /// Updates the `last_seen` timestamp of this peer.
    SetPeerSeen(PeerID, chrono::NaiveDateTime, NetZone),
    /// Bans a peer for the specified duration. This request
    /// will send disconnect signals to all peers with the same
    /// [`ban_identifier`](NetworkAddress::ban_identifier).
    BanPeer(PeerID, std::time::Duration, NetZone),
    /// Adds a peer to the connected list
    ConnectedToPeer {
        /// The net zone of this connection.
        zone: NetZone,
        /// A handle between the connection and address book.
        connection_handle: ConnectionAddressBookHandle,
        /// The connection addr, None if the peer is using a
        /// hidden network.
        addr: Option<NetworkAddress>,
        /// The peers id.
        id: PeerID,
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
    },

    /// A request to get and eempty the anchor list,
    /// used when starting the node.
    GetAndEmptyAnchorList(NetZone),
    /// Get a random Gray peer from the peer list
    /// If a pruning seed is given we will select from
    /// peers with that seed and peers that dont prune.
    GetRandomGrayPeer(NetZone, Option<PruningSeed>),
    /// Get a random White peer from the peer list
    /// If a pruning seed is given we will select from
    /// peers with that seed and peers that dont prune.
    GetRandomWhitePeer(NetZone, Option<PruningSeed>),
    /// Get a list of random peers from the white list,
    /// The list will be less than or equal to the provided
    /// len.
    GetRandomWhitePeers(NetZone, usize),
}

impl std::fmt::Display for AddressBookRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HandleNewPeerList(..) => f.write_str("HandleNewPeerList"),
            Self::SetPeerSeen(..) => f.write_str("SetPeerSeen"),
            Self::BanPeer(..) => f.write_str("BanPeer"),
            Self::ConnectedToPeer { .. } => f.write_str("ConnectedToPeer"),

            Self::GetAndEmptyAnchorList(_) => f.write_str("GetAndEmptyAnchorList"),
            Self::GetRandomGrayPeer(..) => f.write_str("GetRandomGrayPeer"),
            Self::GetRandomWhitePeer(..) => f.write_str("GetRandomWhitePeer"),
            Self::GetRandomWhitePeers(_, len) => {
                f.write_str(&format!("GetRandomWhitePeers, len: {len}"))
            }
        }
    }
}

impl AddressBookRequest {
    /// Gets the [`NetZone`] for this request so we can
    /// route it to the required address book.
    pub fn get_zone(&self) -> NetZone {
        match self {
            Self::HandleNewPeerList(_, zone) => *zone,
            Self::SetPeerSeen(.., zone) => *zone,
            Self::BanPeer(.., zone) => *zone,
            Self::ConnectedToPeer { zone, .. } => *zone,

            Self::GetAndEmptyAnchorList(zone) => *zone,
            Self::GetRandomGrayPeer(zone, _) => *zone,
            Self::GetRandomWhitePeer(zone, _) => *zone,
            Self::GetRandomWhitePeers(zone, _) => *zone,
        }
    }
}

/// A response from the AddressBook.
#[derive(Debug)]
pub enum AddressBookResponse {
    /// The request was handled ok.
    Ok,
    /// A peer.
    Peer(PeerListEntryBase),
    /// A list of peers.
    Peers(Vec<PeerListEntryBase>),
}

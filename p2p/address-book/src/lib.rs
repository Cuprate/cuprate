//! Cuprate Address Book
//!
//! This module holds the logic for persistent peer storage.
//! Cuprates address book is modeled as a [`tower::Service`]
//! The request is [`AddressBookRequest`] and the response is
//! [`AddressBookResponse`].
//!
//! Cuprate, like monerod, actually has multiple address books, one
//! for each [`NetworkZone`]. This is to reduce the possibility of
//! clear net peers getting linked to their dark counterparts
//! and so peers will only get told about peers they can
//! connect to.
//!

use std::{path::PathBuf, time::Duration};

use monero_p2p::{
    services::{AddressBookRequest, AddressBookResponse},
    NetworkZone,
};

mod book;
mod peer_list;
mod store;

#[derive(Debug, Clone)]
pub struct Config {
    max_white_list_length: usize,
    max_gray_list_length: usize,
    peer_store_file: PathBuf,
    peer_save_period: Duration,
}

/// Possible errors when dealing with the address book.
/// This is boxed when returning an error in the [`tower::Service`].
#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum AddressBookError {
    /// The peer is already connected.
    #[error("Peer is already connected")]
    PeerAlreadyConnected,
    /// The peer is not in the address book for this zone.
    #[error("Peer was not found in book")]
    PeerNotFound,
    /// The peer list is empty.
    #[error("The peer list is empty")]
    PeerListEmpty,
    /// Immutable peer data was changed.
    #[error("Immutable peer data was changed: {0}")]
    PeersDataChanged(&'static str),
    /// The peer is banned.
    #[error("The peer is banned")]
    PeerIsBanned,
    /// The channel to the address book has closed unexpectedly.
    #[error("The address books channel has closed.")]
    AddressBooksChannelClosed,
    /// The address book task has exited.
    #[error("The address book task has exited.")]
    AddressBookTaskExited,
}

pub async fn init_address_book<Z: NetworkZone>(
    cfg: Config,
) -> Result<
    impl tower::Service<
        AddressBookRequest<Z>,
        Response = AddressBookResponse<Z>,
        Error = tower::BoxError,
    >,
    std::io::Error,
> {
    let (white_list, gray_list) = store::read_peers_from_disk::<Z>(&cfg).await?;

    let address_book = book::AddressBook::<Z>::new(cfg, white_list, gray_list, Vec::new());

    Ok(tower::buffer::Buffer::new(address_book, 15))
}

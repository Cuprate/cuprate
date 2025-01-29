//! Cuprate Address Book
//!
//! This module holds the logic for persistent peer storage.
//! Cuprates address book is modeled as a [`tower::Service`]
//! The request is [`AddressBookRequest`](cuprate_p2p_core::services::AddressBookRequest) and the response is
//! [`AddressBookResponse`](cuprate_p2p_core::services::AddressBookResponse).
//!
//! Cuprate, like monerod, actually has multiple address books, one
//! for each [`NetworkZone`]. This is to reduce the possibility of
//! clear net peers getting linked to their dark counterparts
//! and so peers will only get told about peers they can
//! connect to.

#![forbid(
    clippy::missing_assert_message,
    clippy::should_panic_without_expect,
    missing_docs,
    unsafe_code,
    reason = "Crate-specific lints. There should be good reasoning when removing these."
)]

use std::{io::ErrorKind, path::PathBuf, time::Duration};

use cuprate_p2p_core::{NetZoneAddress, NetworkZone};

mod book;
mod peer_list;
mod store;

/// The address book config.
#[derive(Debug, Clone)]
pub struct AddressBookConfig {
    /// The maximum number of white peers in the peer list.
    ///
    /// White peers are peers we have connected to before.
    pub max_white_list_length: usize,
    /// The maximum number of gray peers in the peer list.
    ///
    /// Gray peers are peers we are yet to make a connection to.
    pub max_gray_list_length: usize,
    /// The location to store the peer store files.
    pub peer_store_directory: PathBuf,
    /// The amount of time between saving the address book to disk.
    pub peer_save_period: Duration,
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

/// Initializes the P2P address book for a specific network zone.
pub async fn init_address_book<Z: BorshNetworkZone>(
    cfg: AddressBookConfig,
) -> Result<book::AddressBook<Z>, std::io::Error> {
    let (white_list, gray_list) = match store::read_peers_from_disk::<Z>(&cfg).await {
        Ok(res) => res,
        Err(e) if e.kind() == ErrorKind::NotFound => (vec![], vec![]),
        Err(e) => {
            tracing::error!("Failed to open peer list, {}", e);
            panic!("{e}");
        }
    };

    let address_book = book::AddressBook::<Z>::new(cfg, white_list, gray_list, Vec::new());

    Ok(address_book)
}

use sealed::BorshNetworkZone;
mod sealed {
    use super::*;

    /// An internal trait for the address book for a [`NetworkZone`] that adds the requirement of [`borsh`] traits
    /// onto the network address.
    pub trait BorshNetworkZone: NetworkZone<Addr = Self::BorshAddr> {
        type BorshAddr: NetZoneAddress + borsh::BorshDeserialize + borsh::BorshSerialize;
    }

    impl<T: NetworkZone> BorshNetworkZone for T
    where
        T::Addr: borsh::BorshDeserialize + borsh::BorshSerialize,
    {
        type BorshAddr = T::Addr;
    }
}

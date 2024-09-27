//! Functions for TODO: doc enum message.

use std::convert::Infallible;

use anyhow::Error;
use tower::ServiceExt;

use cuprate_helper::cast::usize_to_u64;
use cuprate_p2p_core::{
    services::{AddressBookRequest, AddressBookResponse},
    AddressBook, NetworkZone,
};

/// [`AddressBookRequest::PeerlistSize`]
pub(super) async fn peerlist_size<Z: NetworkZone>(
    address_book: &mut impl AddressBook<Z>,
) -> Result<(u64, u64), Error> {
    let AddressBookResponse::PeerlistSize { white, grey } = address_book
        .ready()
        .await
        .expect("TODO")
        .call(AddressBookRequest::PeerlistSize)
        .await
        .expect("TODO")
    else {
        unreachable!();
    };

    Ok((usize_to_u64(white), usize_to_u64(grey)))
}

/// [`AddressBookRequest::ConnectionCount`]
pub(super) async fn connection_count<Z: NetworkZone>(
    address_book: &mut impl AddressBook<Z>,
) -> Result<(u64, u64), Error> {
    let AddressBookResponse::ConnectionCount { incoming, outgoing } = address_book
        .ready()
        .await
        .expect("TODO")
        .call(AddressBookRequest::ConnectionCount)
        .await
        .expect("TODO")
    else {
        unreachable!();
    };

    Ok((usize_to_u64(incoming), usize_to_u64(outgoing)))
}

/// [`AddressBookRequest::SetBan`]
pub(super) async fn set_ban<Z: NetworkZone>(
    address_book: &mut impl AddressBook<Z>,
    peer: Infallible,
) -> Result<(), Error> {
    let AddressBookResponse::Ok = address_book
        .ready()
        .await
        .expect("TODO")
        .call(AddressBookRequest::SetBan(peer))
        .await
        .expect("TODO")
    else {
        unreachable!();
    };

    Ok(())
}

/// [`AddressBookRequest::GetBan`]
pub(super) async fn get_ban<Z: NetworkZone>(
    address_book: &mut impl AddressBook<Z>,
    peer: Infallible,
) -> Result<(), Error> {
    let AddressBookResponse::GetBan(ban) = address_book
        .ready()
        .await
        .expect("TODO")
        .call(AddressBookRequest::GetBan(peer))
        .await
        .expect("TODO")
    else {
        unreachable!();
    };

    Ok(())
}

/// [`AddressBookRequest::GetBans`]
pub(super) async fn get_bans<Z: NetworkZone>(
    address_book: &mut impl AddressBook<Z>,
) -> Result<(), Error> {
    let AddressBookResponse::GetBans(bans) = address_book
        .ready()
        .await
        .expect("TODO")
        .call(AddressBookRequest::GetBans)
        .await
        .expect("TODO")
    else {
        unreachable!();
    };

    Ok(todo!())
}

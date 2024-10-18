//! Functions for TODO: doc enum message.

use std::convert::Infallible;

use anyhow::{anyhow, Error};
use cuprate_pruning::PruningSeed;
use cuprate_rpc_types::misc::{ConnectionInfo, Span};
use tower::ServiceExt;

use cuprate_helper::cast::usize_to_u64;
use cuprate_p2p_core::{
    services::{AddressBookRequest, AddressBookResponse},
    types::BanState,
    AddressBook, NetworkZone,
};

// FIXME: use `anyhow::Error` over `tower::BoxError` in address book.

/// [`AddressBookRequest::PeerlistSize`]
pub(crate) async fn peerlist_size<Z: NetworkZone>(
    address_book: &mut impl AddressBook<Z>,
) -> Result<(u64, u64), Error> {
    let AddressBookResponse::PeerlistSize { white, grey } = address_book
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(AddressBookRequest::PeerlistSize)
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok((usize_to_u64(white), usize_to_u64(grey)))
}

/// [`AddressBookRequest::ConnectionInfo`]
pub(crate) async fn connection_info<Z: NetworkZone>(
    address_book: &mut impl AddressBook<Z>,
) -> Result<Vec<ConnectionInfo>, Error> {
    let AddressBookResponse::ConnectionInfo(vec) = address_book
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(AddressBookRequest::ConnectionInfo)
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    // FIXME: impl this map somewhere instead of inline.
    let vec = vec
        .into_iter()
        .map(|info| {
            use cuprate_p2p_core::types::AddressType as A1;
            use cuprate_rpc_types::misc::AddressType as A2;

            let address_type = match info.address_type {
                A1::Invalid => A2::Invalid,
                A1::Ipv4 => A2::Ipv4,
                A1::Ipv6 => A2::Ipv6,
                A1::I2p => A2::I2p,
                A1::Tor => A2::Tor,
            };

            ConnectionInfo {
                address: info.address.to_string(),
                address_type,
                avg_download: info.avg_download,
                avg_upload: info.avg_upload,
                connection_id: hex::encode(info.connection_id.to_ne_bytes()),
                current_download: info.current_download,
                current_upload: info.current_upload,
                height: info.height,
                host: info.host,
                incoming: info.incoming,
                ip: info.ip,
                live_time: info.live_time,
                localhost: info.localhost,
                local_ip: info.local_ip,
                peer_id: info.peer_id,
                port: info.port,
                pruning_seed: info.pruning_seed.compress(),
                recv_count: info.recv_count,
                recv_idle_time: info.recv_idle_time,
                rpc_credits_per_hash: info.rpc_credits_per_hash,
                rpc_port: info.rpc_port,
                send_count: info.send_count,
                send_idle_time: info.send_idle_time,
                state: info.state,
                support_flags: info.support_flags,
            }
        })
        .collect();

    Ok(vec)
}

/// [`AddressBookRequest::ConnectionCount`]
pub(crate) async fn connection_count<Z: NetworkZone>(
    address_book: &mut impl AddressBook<Z>,
) -> Result<(u64, u64), Error> {
    let AddressBookResponse::ConnectionCount { incoming, outgoing } = address_book
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(AddressBookRequest::ConnectionCount)
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok((usize_to_u64(incoming), usize_to_u64(outgoing)))
}

/// [`AddressBookRequest::SetBan`]
pub(crate) async fn set_ban<Z: NetworkZone>(
    address_book: &mut impl AddressBook<Z>,
    set_ban: cuprate_p2p_core::types::SetBan<Z::Addr>,
) -> Result<(), Error> {
    let AddressBookResponse::Ok = address_book
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(AddressBookRequest::SetBan(set_ban))
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(())
}

/// [`AddressBookRequest::GetBan`]
pub(crate) async fn get_ban<Z: NetworkZone>(
    address_book: &mut impl AddressBook<Z>,
    peer: Z::Addr,
) -> Result<Option<std::time::Instant>, Error> {
    let AddressBookResponse::GetBan { unban_instant } = address_book
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(AddressBookRequest::GetBan(peer))
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(unban_instant)
}

/// [`AddressBookRequest::GetBans`]
pub(crate) async fn get_bans<Z: NetworkZone>(
    address_book: &mut impl AddressBook<Z>,
) -> Result<Vec<BanState<Z::Addr>>, Error> {
    let AddressBookResponse::GetBans(bans) = address_book
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(AddressBookRequest::GetBans)
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(bans)
}

/// [`AddressBookRequest::Spans`]
pub(crate) async fn spans<Z: NetworkZone>(
    address_book: &mut impl AddressBook<Z>,
) -> Result<Vec<Span>, Error> {
    let AddressBookResponse::Spans(vec) = address_book
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(AddressBookRequest::Spans)
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    // FIXME: impl this map somewhere instead of inline.
    let vec = vec
        .into_iter()
        .map(|span| Span {
            connection_id: hex::encode(span.connection_id.to_ne_bytes()),
            nblocks: span.nblocks,
            rate: span.rate,
            remote_address: span.remote_address.to_string(),
            size: span.size,
            speed: span.speed,
            start_block_height: span.start_block_height,
        })
        .collect();

    Ok(vec)
}

/// [`AddressBookRequest::NextNeededPruningSeed`]
pub(crate) async fn next_needed_pruning_seed<Z: NetworkZone>(
    address_book: &mut impl AddressBook<Z>,
) -> Result<PruningSeed, Error> {
    let AddressBookResponse::NextNeededPruningSeed(seed) = address_book
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(AddressBookRequest::NextNeededPruningSeed)
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    Ok(seed)
}

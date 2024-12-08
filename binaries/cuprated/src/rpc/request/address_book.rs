//! Functions to send [`AddressBookRequest`]s.

use anyhow::{anyhow, Error};
use tower::ServiceExt;

use cuprate_helper::cast::usize_to_u64;
use cuprate_p2p_core::{
    services::{AddressBookRequest, AddressBookResponse},
    types::{BanState, ConnectionId},
    AddressBook, NetworkZone,
};
use cuprate_rpc_types::misc::ConnectionInfo;

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
            let (ip, port) = match info.socket_addr {
                Some(socket) => (socket.ip().to_string(), socket.port().to_string()),
                None => (String::new(), String::new()),
            };

            ConnectionInfo {
                address: info.address.to_string(),
                address_type: info.address_type,
                avg_download: info.avg_download,
                avg_upload: info.avg_upload,
                connection_id: String::from(ConnectionId::DEFAULT_STR),
                current_download: info.current_download,
                current_upload: info.current_upload,
                height: info.height,
                host: info.host,
                incoming: info.incoming,
                ip,
                live_time: info.live_time,
                localhost: info.localhost,
                local_ip: info.local_ip,
                peer_id: hex::encode(info.peer_id.to_ne_bytes()),
                port,
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

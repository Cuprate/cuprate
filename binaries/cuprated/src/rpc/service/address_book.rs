//! Functions to send [`AddressBookRequest`]s.

use std::net::SocketAddrV4;

use anyhow::{anyhow, Error};
use tower::ServiceExt;

use cuprate_helper::{cast::usize_to_u64, map::u32_from_ipv4};
use cuprate_p2p_core::{
    services::{AddressBookRequest, AddressBookResponse, ZoneSpecificPeerListEntryBase},
    types::{BanState, ConnectionId},
    AddressBook, NetworkZone,
};
use cuprate_rpc_types::misc::ConnectionInfo;
use cuprate_types::rpc::Peer;

// FIXME: use `anyhow::Error` over `tower::BoxError` in address book.

/// [`AddressBookRequest::PeerlistSize`]
pub async fn peerlist_size<Z: NetworkZone>(
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
pub async fn connection_info<Z: NetworkZone>(
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
pub async fn connection_count<Z: NetworkZone>(
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
pub async fn set_ban<Z: NetworkZone>(
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
pub async fn get_ban<Z: NetworkZone>(
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
pub async fn get_bans<Z: NetworkZone>(
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

/// [`AddressBookRequest::Peerlist`]
pub async fn peerlist<Z: NetworkZone>(
    address_book: &mut impl AddressBook<Z>,
) -> Result<(Vec<Peer>, Vec<Peer>), Error> {
    let AddressBookResponse::Peerlist(peerlist) = address_book
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(AddressBookRequest::Peerlist)
        .await
        .map_err(|e| anyhow!(e))?
    else {
        unreachable!();
    };

    fn map<Z: NetworkZone>(peers: Vec<ZoneSpecificPeerListEntryBase<Z::Addr>>) -> Vec<Peer> {
        peers
            .into_iter()
            .map(|peer| {
                let ZoneSpecificPeerListEntryBase {
                    adr,
                    id,
                    last_seen,
                    pruning_seed,
                    rpc_port,
                    rpc_credits_per_hash,
                } = peer;

                let host = adr.to_string();

                let (ip, port) = if let Ok(socket_addr) = host.parse::<SocketAddrV4>() {
                    (u32_from_ipv4(*socket_addr.ip()), socket_addr.port())
                } else {
                    (0, 0)
                };

                let last_seen = last_seen.try_into().unwrap_or(0);
                let pruning_seed = pruning_seed.compress();

                Peer {
                    id,
                    host,
                    ip,
                    port,
                    rpc_port,
                    rpc_credits_per_hash,
                    last_seen,
                    pruning_seed,
                }
            })
            .collect()
    }

    let white = map::<Z>(peerlist.white);
    let grey = map::<Z>(peerlist.grey);

    Ok((white, grey))
}

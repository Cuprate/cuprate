//! General data structures.

use std::time::{Duration, Instant};

use cuprate_pruning::PruningSeed;
use cuprate_types::{AddressType, ConnectionState};

use crate::NetZoneAddress;

/// Data within [`crate::services::AddressBookRequest::SetBan`].
pub struct SetBan<A: NetZoneAddress> {
    /// Address of the peer.
    pub address: A,
    /// - If [`Some`], how long this peer should be banned for
    /// - If [`None`], the peer will be unbanned
    pub ban: Option<Duration>,
}

/// Data within [`crate::services::AddressBookResponse::GetBans`].
pub struct BanState<A: NetZoneAddress> {
    /// Address of the peer.
    pub address: A,
    /// - If [`Some`], the peer is banned until this [`Instant`]
    /// - If [`None`], the peer is not currently banned
    pub unban_instant: Option<Instant>,
}

/// Data within [`crate::services::AddressBookResponse::ConnectionInfo`].
pub struct ConnectionInfo<A: NetZoneAddress> {
    // The following fields are mostly the same as `monerod`.
    pub address: A,
    pub address_type: AddressType,
    pub avg_download: u64,
    pub avg_upload: u64,
    pub current_download: u64,
    pub current_upload: u64,
    pub height: u64,
    /// Either a domain or an IP without the port.
    pub host: String,
    pub incoming: bool,
    pub live_time: u64,
    pub localhost: bool,
    pub local_ip: bool,
    pub peer_id: u64,
    pub pruning_seed: PruningSeed,
    pub recv_count: u64,
    pub recv_idle_time: u64,
    pub rpc_credits_per_hash: u32,
    pub rpc_port: u16,
    pub send_count: u64,
    pub send_idle_time: u64,
    pub state: ConnectionState,
    pub support_flags: u32,

    // The following fields are slightly different than `monerod`.
    //
    /// [`None`] if Tor/i2p or unknown.
    pub socket_addr: Option<std::net::SocketAddr>,
    // This field does not exist for Cuprate:
    // <https://github.com/Cuprate/cuprate/pull/320#discussion_r1811335020>
    // pub connection_id: u128,
}

/// Used in RPC's `sync_info`.
///
// TODO: fix docs after <https://github.com/Cuprate/cuprate/pull/320#discussion_r1811089758>
// Data within [`crate::services::AddressBookResponse::Spans`].
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span<A: NetZoneAddress> {
    pub nblocks: u64,
    pub rate: u32,
    pub remote_address: A,
    pub size: u64,
    pub speed: u32,
    pub start_block_height: u64,
}

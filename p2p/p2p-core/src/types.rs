//! General data structures.

use std::time::{Duration, Instant};

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

// TODO: reduce fields and map to RPC type.
//
/// Data within [`crate::services::AddressBookResponse::ConnectionInfo`].
pub struct ConnectionInfo<A: NetZoneAddress> {
    pub address: A,
    pub address_type: u8,
    pub avg_download: u64,
    pub avg_upload: u64,
    pub connection_id: String,
    pub current_download: u64,
    pub current_upload: u64,
    pub height: u64,
    pub host: String,
    pub incoming: bool,
    pub ip: String,
    pub live_time: u64,
    pub localhost: bool,
    pub local_ip: bool,
    pub peer_id: String,
    pub port: String,
    pub pruning_seed: u32,
    pub recv_count: u64,
    pub recv_idle_time: u64,
    pub rpc_credits_per_hash: u32,
    pub rpc_port: u16,
    pub send_count: u64,
    pub send_idle_time: u64,
    pub state: String,
    pub support_flags: u32,
}

/// Used in RPC's `sync_info`.
///
/// Data within [`crate::services::AddressBookResponse::Spans`].
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    pub connection_id: String,
    pub nblocks: u64,
    pub rate: u32,
    pub remote_address: String,
    pub size: u64,
    pub speed: u32,
    pub start_block_height: u64,
}

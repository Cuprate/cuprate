//! Data structures related to bans.

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

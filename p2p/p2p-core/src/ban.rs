//! Data structures related to bans.

use std::time::{Duration, Instant};

use crate::NetZoneAddress;

/// Data within [`crate::services::AddressBookRequest::SetBan`].
pub struct SetBan<A: NetZoneAddress> {
    pub address: A,
    pub ban: bool,
    pub duration: Duration,
}

/// Data within [`crate::services::AddressBookResponse::GetBans`].
pub struct BanState<A: NetZoneAddress> {
    pub address: A,
    pub banned: bool,
    pub unban_instant: Instant,
}

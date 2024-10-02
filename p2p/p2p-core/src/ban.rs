//! Data structures related to bans.

use std::time::{Duration, Instant};

use crate::NetZoneAddress;

/// TODO
pub struct SetBan<A: NetZoneAddress> {
    pub address: A,
    pub ban: bool,
    pub duration: Duration,
}

/// TODO
pub struct BanState<A: NetZoneAddress> {
    pub address: A,
    pub banned: bool,
    pub unban_instant: Instant,
}

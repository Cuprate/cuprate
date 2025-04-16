//! Networking utilities.
//!
//! `#[no_std]` compatible.

use core::net::IpAddr;

/// Returns [`true`] if the address is a local address
/// (non-reachable via the broader internet).
///
/// # FIXME
/// This is only mostly accurate.
///
/// It should be replaced when `std` stabilizes things:
/// <https://github.com/rust-lang/rust/issues/27709>
pub const fn ip_is_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_loopback() || ip.is_private(),
        IpAddr::V6(ip) => ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local(),
    }
}

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

#[cfg(test)]
mod test {
    use core::net::{Ipv4Addr, Ipv6Addr};

    use super::*;

    #[test]
    fn ip_local() {
        for ipv4 in [
            Ipv4Addr::LOCALHOST,
            Ipv4Addr::new(10, 0, 0, 0),
            Ipv4Addr::new(10, 0, 255, 255),
            Ipv4Addr::new(172, 16, 0, 0),
            Ipv4Addr::new(172, 16, 255, 255),
            Ipv4Addr::new(192, 168, 0, 0),
            Ipv4Addr::new(192, 168, 255, 255),
        ] {
            assert!(ip_is_local(ipv4.into()));
        }

        for ipv4 in [
            Ipv4Addr::UNSPECIFIED,
            Ipv4Addr::new(1, 1, 1, 1),
            Ipv4Addr::new(176, 9, 0, 187),
            Ipv4Addr::new(88, 198, 163, 90),
            Ipv4Addr::new(66, 85, 74, 134),
            Ipv4Addr::new(51, 79, 173, 165),
            Ipv4Addr::new(192, 99, 8, 110),
            Ipv4Addr::new(37, 187, 74, 171),
            Ipv4Addr::new(77, 172, 183, 193),
        ] {
            assert!(!ip_is_local(ipv4.into()));
        }

        for ipv6 in [
            Ipv6Addr::LOCALHOST,
            Ipv6Addr::new(0xfc02, 0, 0, 0, 0, 0, 0, 0),
            Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0),
            Ipv6Addr::new(0xfe80, 0, 0, 1, 0, 0, 0, 0),
            Ipv6Addr::new(0xfe81, 0, 0, 0, 0, 0, 0, 0),
        ] {
            assert!(ip_is_local(ipv6.into()));
        }

        for ipv6 in [
            Ipv6Addr::UNSPECIFIED,
            Ipv6Addr::new(1, 1, 1, 1, 1, 1, 1, 1),
            Ipv6Addr::new(
                0x1020, 0x3040, 0x5060, 0x7080, 0x90A0, 0xB0C0, 0xD0E0, 0xF00D,
            ),
        ] {
            assert!(!ip_is_local(ipv6.into()));
        }
    }
}

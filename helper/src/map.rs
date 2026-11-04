//! Mapping of data types.
//!
//! This module provides functions solely for mapping data types into others, mostly similar ones.
//!
//! `#[no_std]` compatible.

//---------------------------------------------------------------------------------------------------- Use
use core::net::Ipv4Addr;

//---------------------------------------------------------------------------------------------------- `(u64, u64) <-> u128`
/// Split a [`u128`] value into 2 64-bit values.
///
/// The tuple returned is `(low, high)` where `low` is the least significant
/// 64-bits of `number`, and `high` is the most significant.
///
/// Note that the output of this function are `u64` representations of _bits_, not numerical values.
///
/// See [`combine_low_high_bits_to_u128`] for the inverse function.
///
/// ```rust
/// # use cuprate_helper::map::*;
/// let value = u128::MAX - 1;
/// let low = u64::MAX - 1;
/// let high = u64::MAX;
///
/// assert_eq!(split_u128_into_low_high_bits(value), (low, high));
/// assert_eq!(split_u128_into_low_high_bits(0), (0, 0));
/// ```
#[inline]
pub const fn split_u128_into_low_high_bits(value: u128) -> (u64, u64) {
    #[expect(clippy::cast_possible_truncation)]
    (value as u64, (value >> 64) as u64)
}

/// Combine 2 64-bit values into a single [`u128`] value.
///
/// The inputs:
/// - `low_bits` are the _least_ significant 64-bits of `cumulative_difficulty`
/// - `high_bits` are the _most_ significant 64-bits of `cumulative_difficulty`
///
/// Note that `low_bits` & `high_bits` should be `u64` representation of _bits_, not numerical values.
///
/// See [`split_u128_into_low_high_bits`] for the inverse function.
///
/// ```rust
/// # use cuprate_helper::map::*;
/// let value = u128::MAX - 1;
/// let low = u64::MAX - 1;
/// let high = u64::MAX;
///
/// assert_eq!(combine_low_high_bits_to_u128(low, high), value);
/// assert_eq!(combine_low_high_bits_to_u128(0, 0), 0);
/// ```
#[inline]
pub const fn combine_low_high_bits_to_u128(low_bits: u64, high_bits: u64) -> u128 {
    let res = (high_bits as u128) << 64;
    res | (low_bits as u128)
}

//---------------------------------------------------------------------------------------------------- IPv4
/// Convert an [`Ipv4Addr`] to a [`u32`].
///
/// For why this exists, see: <https://architecture.cuprate.org/oddities/le-ipv4.html>.
#[inline]
pub const fn ipv4_from_u32(ip: u32) -> Ipv4Addr {
    let [a, b, c, d] = ip.to_le_bytes();
    Ipv4Addr::new(a, b, c, d)
}

/// Convert a [`u32`] to an [`Ipv4Addr`].
///
/// For why this exists, see: <https://architecture.cuprate.org/oddities/le-ipv4.html>.
#[inline]
pub const fn u32_from_ipv4(ip: Ipv4Addr) -> u32 {
    u32::from_le_bytes(ip.octets())
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}

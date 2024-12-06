//! Formatting.

use crate::map::combine_low_high_bits_to_u128;

/// Format two [`u64`]'s as a [`u128`] as a lower-case hexadecimal string prefixed with `0x`.
///
/// ```rust
/// # use cuprate_helper::fmt::hex_prefix_u128;
/// assert_eq!(hex_prefix_u128(0, 0), "0x0");
/// assert_eq!(hex_prefix_u128(0, u64::MAX), "0xffffffffffffffff0000000000000000");
/// assert_eq!(hex_prefix_u128(u64::MAX, 0), "0xffffffffffffffff");
/// assert_eq!(hex_prefix_u128(u64::MAX, u64::MAX), "0xffffffffffffffffffffffffffffffff");
/// ```
pub fn hex_prefix_u128(low_bits: u64, high_bits: u64) -> String {
    format!("{:#x}", combine_low_high_bits_to_u128(low_bits, high_bits))
}

#[cfg(test)]
mod tests {}

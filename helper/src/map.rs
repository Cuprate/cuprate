//! Mapping of data types.
//!
//! This module provides functions solely for mapping data types into others, mostly similar ones.
//!
//! `#[no_std]` compatible.

//---------------------------------------------------------------------------------------------------- Use

//---------------------------------------------------------------------------------------------------- cumulative_difficulty
/// Split a 128-bit `cumulative_difficulty` value into low and high bits.
///
/// The tuple returned is `(low, high)` where `low` is the least significant
/// 64-bits of `cumulative_difficulty`, and `high` is the most significant.
///
/// See [`cumulative_difficulty_from_low_high_bits`] for the inverse function.
///
/// ```rust
/// # use cuprate_helper::map::*;
/// //                            cumulative_difficulty          low         high
/// //                                                v            v         v
/// assert_eq!(cumulative_difficulty_to_low_high_bits(u128::MAX), (u64::MAX, u64::MAX));
/// ```
pub const fn cumulative_difficulty_to_low_high_bits(cumulative_difficulty: u128) -> (u64, u64) {
    let bits = cumulative_difficulty.to_le_bytes();

    let low = u64::from_le_bytes([
        bits[0], bits[1], bits[2], bits[3], bits[4], bits[5], bits[6], bits[7],
    ]);
    let high = u64::from_le_bytes([
        bits[8], bits[9], bits[10], bits[11], bits[12], bits[13], bits[14], bits[15],
    ]);

    (low, high)
}

/// Combine 2 64-bit values into a single 128-bit `cumulative_difficulty`.
///
/// The inputs:
/// - `low_bits` are the _least_ significant 64-bits of `cumulative_difficulty`
/// - `high_bits` are the _most_ significant 64-bits of `cumulative_difficulty`
///
/// See [`cumulative_difficulty_to_low_high_bits`] for the inverse function.
///
/// ```rust
/// # use cuprate_helper::map::*;
/// //                                              low         high       cumulative_difficulty
/// //                                                v         v          v
/// assert_eq!(cumulative_difficulty_from_low_high_bits(u64::MAX, u64::MAX), u128::MAX);
/// ```
pub const fn cumulative_difficulty_from_low_high_bits(low_bits: u64, high_bits: u64) -> u128 {
    let low = low_bits.to_le_bytes();
    let high = high_bits.to_le_bytes();

    u128::from_le_bytes([
        low[0], low[1], low[2], low[3], low[4], low[5], low[6], low[7], high[0], high[1], high[2],
        high[3], high[4], high[5], high[6], high[7],
    ])
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}

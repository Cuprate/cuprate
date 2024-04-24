//! Mapping of data types.
//!
//! This module provides functions solely for mapping data types into others, mostly similar ones.
//!
//! `#[no_std]` compatible.

//---------------------------------------------------------------------------------------------------- Use

//---------------------------------------------------------------------------------------------------- cumulative_difficulty
/// Split a [`u128`] numerical value into 2 64-bit values.
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
/// //                              regular u128 number          low         high
/// //                                                v            v         v
/// assert_eq!(split_u128_into_low_high_bits(u128::MAX), (u64::MAX, u64::MAX));
/// ```
#[inline]
pub const fn split_u128_into_low_high_bits(number: u128) -> (u64, u64) {
    let bits = number.to_le_bytes();

    let low = u64::from_le_bytes([
        bits[0], bits[1], bits[2], bits[3], bits[4], bits[5], bits[6], bits[7],
    ]);
    let high = u64::from_le_bytes([
        bits[8], bits[9], bits[10], bits[11], bits[12], bits[13], bits[14], bits[15],
    ]);

    (low, high)
}

/// Combine 2 64-bit values into a single [`u128`] numerical value.
///
/// The inputs:
/// - `low_bits` are the _least_ significant 64-bits of `cumulative_difficulty`
/// - `high_bits` are the _most_ significant 64-bits of `cumulative_difficulty`
///
/// Note that `low_bits` & `high_bits` should be `u64` representation of _bits_, not numerical values.
///
/// Although, the output of this function is a _numerical_ `u128` value.
///
/// See [`split_u128_into_low_high_bits`] for the inverse function.
///
/// ```rust
/// # use cuprate_helper::map::*;
/// //                                     low         high       regular u128 value
/// //                                       v         v          v
/// assert_eq!(combine_low_high_bits_to_u128(u64::MAX, u64::MAX), u128::MAX);
/// ```
#[inline]
pub const fn combine_low_high_bits_to_u128(low_bits: u64, high_bits: u64) -> u128 {
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

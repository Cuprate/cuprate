//! Mapping of data types.
//!
//! This module provides functions solely for mapping data types into others, mostly similar ones.
//!
//! `#[no_std]` compatible.

//---------------------------------------------------------------------------------------------------- Use
use monero_serai::transaction::Timelock;

use cuprate_constants::block::MAX_BLOCK_HEIGHT;

use crate::cast::{u64_to_usize, usize_to_u64};

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
/// ```
#[inline]
pub const fn combine_low_high_bits_to_u128(low_bits: u64, high_bits: u64) -> u128 {
    let res = (high_bits as u128) << 64;
    res | (low_bits as u128)
}

//---------------------------------------------------------------------------------------------------- Timelock
/// Map a [`u64`] to a [`Timelock`].
///
/// Height/time is not differentiated via type, but rather:
/// "height is any value less than [`MAX_BLOCK_HEIGHT`] and timestamp is any value above"
/// so the `u64/usize` is stored without any tag.
///
/// See [`timelock_to_u64`] for the inverse function.
///
/// - <https://github.com/Cuprate/cuprate/pull/102#discussion_r1558504285>
/// - <https://github.com/serai-dex/serai/blob/bc1dec79917d37d326ac3d9bc571a64131b0424a/coins/monero/src/transaction.rs#L139>
///
/// ```rust
/// # use cuprate_helper::map::*;
/// # use monero_serai::transaction::*;
/// use cuprate_constants::block::MAX_BLOCK_HEIGHT;
/// assert_eq!(u64_to_timelock(0), Timelock::None);
/// assert_eq!(u64_to_timelock(MAX_BLOCK_HEIGHT-1), Timelock::Block(MAX_BLOCK_HEIGHT-1));
/// assert_eq!(u64_to_timelock(MAX_BLOCK_HEIGHT), Timelock::Time(MAX_BLOCK_HEIGHT));
/// ```
pub const fn u64_to_timelock(u: u64) -> Timelock {
    if u == 0 {
        Timelock::None
    } else if u < MAX_BLOCK_HEIGHT {
        Timelock::Block(u64_to_usize(u))
    } else {
        Timelock::Time(u)
    }
}

/// Map [`Timelock`] to a [`u64`].
///
/// See [`u64_to_timelock`] for the inverse function and more documentation.
///
/// ```rust
/// # use cuprate_helper::map::*;
/// # use monero_serai::transaction::*;
/// use cuprate_constants::block::MAX_BLOCK_HEIGHT;
/// assert_eq!(timelock_to_u64(Timelock::None), 0);
/// assert_eq!(timelock_to_u64(Timelock::Block(MAX_BLOCK_HEIGHT-1)), MAX_BLOCK_HEIGHT-1);
/// assert_eq!(timelock_to_u64(Timelock::Time(MAX_BLOCK_HEIGHT)), MAX_BLOCK_HEIGHT);
/// ```
pub const fn timelock_to_u64(timelock: Timelock) -> u64 {
    match timelock {
        Timelock::None => 0,
        Timelock::Block(u) => usize_to_u64(u),
        Timelock::Time(u) => u,
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}

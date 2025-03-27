//! Casting.
//!
//! This modules provides utilities for casting between types.
//!
//! `#[no_std]` compatible.
//!
//! # 64-bit invariant
//! This module is available on 32-bit arches although panics
//! will occur between lossy casts, e.g. [`u64_to_usize`] where
//! the input is larger than [`u32::MAX`].
//!
//! On 64-bit arches, all functions are lossless.

// TODO:
// These casting functions are heavily used throughout the codebase
// yet it is not enforced that all usages are correct in 32-bit cases.
// Panicking may be a short-term solution - find a better fix for 32-bit arches.

#![allow(clippy::cast_possible_truncation)]

#[rustfmt::skip]
//============================ SAFETY: DO NOT REMOVE ===========================//
//                                                                              //
//                                                                              //
//                   Only allow building {32,64}-bit targets.                   //
//          This allows us to assume {32,64}-bit invariants in this file.       //
    #[cfg(not(any(target_pointer_width = "64", target_pointer_width = "32")))]
      compile_error!("This module is only compatible with {32,64}-bit CPUs");
//                                                                              //
//                                                                              //
//============================ SAFETY: DO NOT REMOVE ===========================//

#[cfg(target_pointer_width = "64")]
mod functions {
    /// Cast [`u64`] to [`usize`].
    #[inline(always)]
    pub const fn u64_to_usize(u: u64) -> usize {
        u as usize
    }

    /// Cast [`i64`] to [`isize`].
    #[inline(always)]
    pub const fn i64_to_isize(i: i64) -> isize {
        i as isize
    }
}

#[cfg(target_pointer_width = "32")]
mod functions {
    /// Cast [`u64`] to [`usize`].
    ///
    /// # Panics
    /// This panics on 32-bit arches if `u` is larger than [`u32::MAX`].
    #[inline(always)]
    pub const fn u64_to_usize(u: u64) -> usize {
        if u > u32::MAX as u64 {
            panic!()
        } else {
            u as usize
        }
    }

    /// Cast [`i64`] to [`isize`].
    ///
    /// # Panics
    /// This panics on 32-bit arches if `i` is lesser than [`i32::MIN`] or greater [`i32::MAX`].
    #[inline(always)]
    pub const fn i64_to_isize(i: i64) -> isize {
        if i < i32::MIN as i64 || i > i32::MAX as i64 {
            panic!()
        } else {
            i as isize
        }
    }
}

pub use functions::{i64_to_isize, u64_to_usize};

/// Cast [`u32`] to [`usize`].
#[inline(always)]
pub const fn u32_to_usize(u: u32) -> usize {
    u as usize
}

/// Cast [`i32`] to [`isize`].
#[inline(always)]
pub const fn i32_to_isize(i: i32) -> isize {
    i as isize
}

/// Cast [`usize`] to [`u64`].
#[inline(always)]
pub const fn usize_to_u64(u: usize) -> u64 {
    u as u64
}

/// Cast [`isize`] to [`i64`].
#[inline(always)]
pub const fn isize_to_i64(i: isize) -> i64 {
    i as i64
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn max_64bit() {
        assert_eq!(u32_to_usize(u32::MAX), usize::try_from(u32::MAX).unwrap());
        assert_eq!(usize_to_u64(u32_to_usize(u32::MAX)), u64::from(u32::MAX));

        assert_eq!(u64_to_usize(u64::MAX), usize::MAX);
        assert_eq!(usize_to_u64(u64_to_usize(u64::MAX)), u64::MAX);

        assert_eq!(usize_to_u64(usize::MAX), u64::MAX);
        assert_eq!(u64_to_usize(usize_to_u64(usize::MAX)), usize::MAX);

        assert_eq!(i32_to_isize(i32::MAX), isize::try_from(i32::MAX).unwrap());
        assert_eq!(isize_to_i64(i32_to_isize(i32::MAX)), i64::from(i32::MAX));

        assert_eq!(i64_to_isize(i64::MAX), isize::MAX);
        assert_eq!(isize_to_i64(i64_to_isize(i64::MAX)), i64::MAX);

        assert_eq!(isize_to_i64(isize::MAX), i64::MAX);
        assert_eq!(i64_to_isize(isize_to_i64(isize::MAX)), isize::MAX);
    }

    #[test]
    #[cfg(target_pointer_width = "32")]
    #[should_panic]
    fn panic_u64_32bit() {
        u64_to_usize(u64::from(u32::MAX + 1));
    }

    #[test]
    #[cfg(target_pointer_width = "32")]
    #[should_panic]
    fn panic_i64_lesser_32bit() {
        i64_to_usize(i64::from(i32::MIN - 1));
    }

    #[test]
    #[cfg(target_pointer_width = "32")]
    #[should_panic]
    fn panic_i64_greater_32bit() {
        i64_to_usize(i64::from(i32::MAX + 1));
    }
}

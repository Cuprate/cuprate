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
}

pub use functions::u64_to_usize;

/// Cast [`u32`] to [`usize`].
#[inline(always)]
pub const fn u32_to_usize(u: u32) -> usize {
    u as usize
}

/// Cast [`usize`] to [`u64`].
#[inline(always)]
pub const fn usize_to_u64(u: usize) -> u64 {
    u as u64
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
    }

    #[test]
    #[cfg(target_pointer_width = "32")]
    #[should_panic]
    fn panic_u64_32bit() {
        u64_to_usize(u64::from(u32::MAX + 1));
    }
}

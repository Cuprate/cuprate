//! Casting.
//!
//! This modules provides utilities for casting between types.
//!
//! `#[no_std]` compatible.

#![allow(clippy::cast_possible_truncation)]

#[rustfmt::skip]
//============================ SAFETY: DO NOT REMOVE ===========================//
//                                                                              //
//                                                                              //
//                     Only allow building 64-bit targets.                      //
//            This allows us to assume 64-bit invariants in this file.          //
                       #[cfg(not(target_pointer_width = "64"))]
           compile_error!("Cuprate is only compatible with 64-bit CPUs");
//                                                                              //
//                                                                              //
//============================ SAFETY: DO NOT REMOVE ===========================//

//---------------------------------------------------------------------------------------------------- Free functions
/// Cast [`u64`] to [`usize`].
#[inline(always)]
pub const fn u64_to_usize(u: u64) -> usize {
    u as usize
}

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

/// Cast [`i64`] to [`isize`].
#[inline(always)]
pub const fn i64_to_isize(i: i64) -> isize {
    i as isize
}

/// Cast [`i32`] to [`isize`].
#[inline(always)]
pub const fn i32_to_isize(i: i32) -> isize {
    i as isize
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
    fn max_unsigned() {
        assert_eq!(u32_to_usize(u32::MAX), usize::try_from(u32::MAX).unwrap());
        assert_eq!(usize_to_u64(u32_to_usize(u32::MAX)), u64::from(u32::MAX));

        assert_eq!(u64_to_usize(u64::MAX), usize::MAX);
        assert_eq!(usize_to_u64(u64_to_usize(u64::MAX)), u64::MAX);

        assert_eq!(usize_to_u64(usize::MAX), u64::MAX);
        assert_eq!(u64_to_usize(usize_to_u64(usize::MAX)), usize::MAX);
    }

    #[test]
    fn max_signed() {
        assert_eq!(i32_to_isize(i32::MAX), isize::try_from(i32::MAX).unwrap());
        assert_eq!(isize_to_i64(i32_to_isize(i32::MAX)), i64::from(i32::MAX));

        assert_eq!(i64_to_isize(i64::MAX), isize::MAX);
        assert_eq!(isize_to_i64(i64_to_isize(i64::MAX)), i64::MAX);

        assert_eq!(isize_to_i64(isize::MAX), i64::MAX);
        assert_eq!(i64_to_isize(isize_to_i64(isize::MAX)), isize::MAX);
    }
}

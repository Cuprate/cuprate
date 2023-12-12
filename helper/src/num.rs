//! Number related
//!

//---------------------------------------------------------------------------------------------------- Use
use std::ops::{Add, Div, Mul, Sub};

//---------------------------------------------------------------------------------------------------- Public API
#[inline]
/// Returns the average of two numbers; works with at least all integral and floating point types
///
/// ```rust
/// # use helper::num::*;
/// assert_eq!(get_mid(0,        10),       5);
/// assert_eq!(get_mid(0.0,      10.0),     5.0);
/// assert_eq!(get_mid(-10.0,    10.0),     0.0);
/// assert_eq!(get_mid(i16::MIN, i16::MAX), -1);
/// assert_eq!(get_mid(u8::MIN,  u8::MAX),  127);
///
/// assert!(get_mid(f32::NAN, f32::NAN).is_nan());
/// assert!(get_mid(f32::NEG_INFINITY, f32::INFINITY).is_nan());
/// ```
pub fn get_mid<T>(a: T, b: T) -> T
where
    T: Add<Output = T> + Sub<Output = T> + Div<Output = T> + Mul<Output = T> + Copy + From<u8>,
{
    let two: T = 2_u8.into();

    // https://github.com/monero-project/monero/blob/90294f09ae34ef96f3dea5fea544816786df87c8/contrib/epee/include/misc_language.h#L43
    (a / two) + (b / two) + ((a - two * (a / two)) + (b - two * (b / two))) / two
}

#[inline]
/// Gets the median from a sorted slice.
///
/// ```rust
/// # use helper::num::*;
/// let mut vec = vec![10, 5, 1, 4, 2, 8, 9, 7, 3, 6];
/// vec.sort();
///
/// assert_eq!(median(vec), 5);
/// ```
///
/// # Safety
/// If not sorted the output will be invalid.
pub fn median<T>(array: impl AsRef<[T]>) -> T
where
    T: Add<Output = T> + Sub<Output = T> + Div<Output = T> + Mul<Output = T> + Copy + From<u8>,
{
    let array = array.as_ref();
    let len = array.len();

    let mid = len / 2;

    if len == 1 {
        return array[0];
    }

    if len % 2 == 0 {
        get_mid(array[mid - 1], array[mid])
    } else {
        array[mid]
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}

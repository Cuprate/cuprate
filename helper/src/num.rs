//! Number related
//!
//! `#[no_std]` compatible.

//---------------------------------------------------------------------------------------------------- Use
use core::{
    cmp::Ordering,
    ops::{Add, Div, Mul, Sub},
};

#[cfg(feature = "std")]
mod rolling_median;

//---------------------------------------------------------------------------------------------------- Types
// INVARIANT: must be private.
// Protects against outside-crate implementations.
mod private {
    pub trait Sealed: Copy + PartialOrd<Self> + core::fmt::Display {}
}

#[cfg(feature = "std")]
pub use rolling_median::RollingMedian;

/// Non-floating point numbers
///
/// This trait is sealed and is only implemented on:
/// - [`u8`] to [`u128`] and [`usize`]
/// - [`i8`] to [`i128`] and [`isize`]
pub trait Number: private::Sealed {}
macro_rules! impl_number {
    ($($num:ty),* $(,)?) => {
        $(
            impl Number for $num {}
            impl private::Sealed for $num {}
        )*
    };
}
impl_number!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

/// Floating point numbers
///
/// This trait is sealed and is only implemented on:
/// - [`f32`]
/// - [`f64`]
pub trait Float: private::Sealed {}
macro_rules! impl_float {
    ($($num:ty),* $(,)?) => {
        $(
            impl Float for $num {}
            impl private::Sealed for $num {}
        )*
    };
}
impl_float!(f32, f64);

//---------------------------------------------------------------------------------------------------- Free Functions
#[inline]
/// Returns the average of two numbers; works with at least all integral and floating point types
///
/// ```rust
/// # use cuprate_helper::num::*;
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
/// # use cuprate_helper::num::*;
/// let mut vec = vec![10, 5, 1, 4, 2, 8, 9, 7, 3, 6];
/// vec.sort();
///
/// assert_eq!(median(vec), 5);
/// ```
///
/// # Invariant
/// If not sorted the output will be invalid.
#[expect(clippy::debug_assert_with_mut_call)]
pub fn median<T>(array: impl AsRef<[T]>) -> T
where
    T: Add<Output = T>
        + Sub<Output = T>
        + Div<Output = T>
        + Mul<Output = T>
        + PartialOrd
        + Copy
        + From<u8>,
{
    let array = array.as_ref();
    let len = array.len();

    // TODO: use `is_sorted` when stable.
    debug_assert!(array
        .windows(2)
        .try_for_each(|window| if window[0] <= window[1] {
            Ok(())
        } else {
            Err(())
        })
        .is_ok());

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

#[inline]
/// Compare 2 non-`NaN` floats.
///
/// ```rust
/// # use cuprate_helper::num::*;
/// # use core::cmp::Ordering;
/// assert_eq!(cmp_float(0.0, 1.0), Ordering::Less);
/// assert_eq!(cmp_float(1.0, 1.0), Ordering::Equal);
/// assert_eq!(cmp_float(2.0, 1.0), Ordering::Greater);
///
/// assert_eq!(cmp_float(1.0,           f32::INFINITY), Ordering::Less);
/// assert_eq!(cmp_float(f32::INFINITY, f32::INFINITY), Ordering::Equal);
/// assert_eq!(cmp_float(f32::INFINITY, 1.0),           Ordering::Greater);
///
/// assert_eq!(cmp_float(f32::NEG_INFINITY, f32::INFINITY),     Ordering::Less);
/// assert_eq!(cmp_float(f32::NEG_INFINITY, f32::NEG_INFINITY), Ordering::Equal);
/// assert_eq!(cmp_float(f32::INFINITY,     f32::NEG_INFINITY), Ordering::Greater);
/// ```
///
/// # Panic
/// This function panics if either floats are NaNs.
///
/// ```rust,should_panic
/// # use cuprate_helper::num::*;
/// cmp_float(0.0, f32::NAN);
/// ```
pub fn cmp_float<F: Float>(a: F, b: F) -> Ordering {
    match (a <= b, a >= b) {
        (false, true) => Ordering::Greater,
        (true, false) => Ordering::Less,
        (true, true) => Ordering::Equal,
        _ => panic!("cmp_float() has failed, input: {a} - {b}"),
    }
}

#[inline]
/// Compare 2 floats, `NaN`'s will always return [`Ordering::Equal`].
///
/// ```rust
/// # use cuprate_helper::num::*;
/// # use core::cmp::Ordering;
/// assert_eq!(cmp_float_nan(0.0, 1.0), Ordering::Less);
/// assert_eq!(cmp_float_nan(1.0, 1.0), Ordering::Equal);
/// assert_eq!(cmp_float_nan(2.0, 1.0), Ordering::Greater);
///
/// assert_eq!(cmp_float_nan(1.0,           f32::INFINITY), Ordering::Less);
/// assert_eq!(cmp_float_nan(f32::INFINITY, f32::INFINITY), Ordering::Equal);
/// assert_eq!(cmp_float_nan(f32::INFINITY, 1.0),           Ordering::Greater);
///
/// assert_eq!(cmp_float_nan(f32::NEG_INFINITY, f32::INFINITY),     Ordering::Less);
/// assert_eq!(cmp_float_nan(f32::NEG_INFINITY, f32::NEG_INFINITY), Ordering::Equal);
/// assert_eq!(cmp_float_nan(f32::INFINITY,     f32::NEG_INFINITY), Ordering::Greater);
///
/// assert_eq!(cmp_float_nan(f32::NAN, -0.0),              Ordering::Equal);
/// assert_eq!(cmp_float_nan(f32::NAN, 0.0),               Ordering::Equal);
/// assert_eq!(cmp_float_nan(f32::NAN, f32::NAN),          Ordering::Equal);
/// assert_eq!(cmp_float_nan(f32::NAN, f32::INFINITY),     Ordering::Equal);
/// assert_eq!(cmp_float_nan(f32::NAN, f32::NEG_INFINITY), Ordering::Equal);
/// ```
pub fn cmp_float_nan<F: Float>(a: F, b: F) -> Ordering {
    match (a <= b, a >= b) {
        (false, true) => Ordering::Greater,
        (true, false) => Ordering::Less,
        _ => Ordering::Equal,
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {}

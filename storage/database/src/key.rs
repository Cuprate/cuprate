//! Database key abstraction; `trait Key`.

//---------------------------------------------------------------------------------------------------- Import
use std::cmp::Ordering;

use crate::{storable::Storable, StorableBytes, StorableVec};

//---------------------------------------------------------------------------------------------------- Table
/// Database [`Table`](crate::table::Table) key metadata.
///
/// Purely compile time information for database table keys.
//
// FIXME: this doesn't need to exist right now but
// may be used if we implement getting values using ranges.
// <https://github.com/Cuprate/cuprate/pull/117#discussion_r1589378104>
pub trait Key: Storable + Sized {
    /// The primary key type.
    type Primary: Storable;

    /// Compare 2 [`Key`]'s against each other.
    ///
    /// # Defaults
    /// For arrays and vectors that contain a `T: Storable`,
    /// this does a straight _byte_ comparison, not a comparison of the key's value.
    ///
    /// For numbers ([`u8`], [`i128`], etc), this will attempt to decode
    /// the number from the bytes, then do a number comparison.
    ///
    /// In the number case, functions like [`u8::from_ne_bytes`] are used,
    /// since [`Storable`] doesn't give any guarantees about endianness.
    ///
    /// # Example
    /// ```rust
    /// # use cuprate_database::*;
    /// // Normal byte comparison.
    /// let vec1 = StorableVec(vec![0]);
    /// let vec2 = StorableVec(vec![2]);
    /// assert_eq!(
    ///     <StorableVec<u8> as Key>::compare(&vec1, &vec2),
    ///     std::cmp::Ordering::Less,
    /// );
    ///
    /// // Integer comparison.
    /// let num1 = i8::to_ne_bytes(1);
    /// let num2 = i8::to_ne_bytes(-1);
    /// assert_eq!(
    ///     <i8 as Key>::compare(&num1, &num2),
    ///     std::cmp::Ordering::Greater,
    /// );
    /// ```
    #[inline]
    fn compare(left: &[u8], right: &[u8]) -> Ordering {
        left.cmp(right)
    }
}

//---------------------------------------------------------------------------------------------------- Free
// [`Ord`] comparison for arrays.
impl<const N: usize, T> Key for [T; N]
where
    T: Key + Storable + Sized + bytemuck::Pod,
{
    type Primary = Self;
}

/// [`Ord`] comparison for vectors.
macro_rules! impl_key_cmp {
    ($($t:ty),* $(,)?) => {
        $(
            impl Key for $t {
                type Primary = Self;
            }
        )*
    };
}
impl_key_cmp! {
    StorableBytes,
    StorableVec<u8>,StorableVec<u16>,StorableVec<u32>,StorableVec<u64>,StorableVec<u128>,
    StorableVec<i8>,StorableVec<i16>,StorableVec<i32>,StorableVec<i64>,StorableVec<i128>,
    StorableVec<f32>,StorableVec<f64>,
}

/// Integer comparison for numbers.
macro_rules! impl_key_ne_bytes {
    ($($t:ident),* $(,)?) => {
        $(
            impl Key for $t {
                type Primary = Self;

                fn compare(left: &[u8], right: &[u8]) -> Ordering {
                    let left = $t::from_ne_bytes(left.try_into().unwrap());
                    let right = $t::from_ne_bytes(right.try_into().unwrap());
                    std::cmp::Ord::cmp(&left, &right)
                }
            }
        )*
    };
}

impl_key_ne_bytes! {
    u8,u16,u32,u64,u128,
    i8,i16,i32,i64,i128,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

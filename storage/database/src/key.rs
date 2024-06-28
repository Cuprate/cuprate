//! Database key abstraction; `trait Key`.

//---------------------------------------------------------------------------------------------------- Import
use std::{cmp::Ordering, fmt::Debug};

use crate::{storable::Storable, StorableBytes, StorableStr, StorableVec};

//---------------------------------------------------------------------------------------------------- Table
/// Database [`Table`](crate::table::Table) key metadata.
///
/// Purely compile time information for database table keys.
///
/// ## Comparison
/// There are 2 differences between [`Key`] and [`Storable`]:
/// 1. [`Key`] must be [`Sized`]
/// 2. [`Key`] represents a [`Storable`] type that defines a comparison function
///
/// The database backends will use [`Key::compare`] to sort the keys
/// within database tables.
///
/// [`Key::compare`] is pre-implemented as a straight byte comparison.
///
/// This default is overridden for numbers, which use a number comparison.
/// For example, [`u64`] keys are sorted as `{0, 1, 2 ... 999_998, 999_999, 1_000_000}`.
///
/// If you would like to re-define this for number types, consider creating a
/// wrapper type around primitives like a `struct SortU8(pub u8)` and implement
/// [`Storable`], [`Key`], and define a custom [`Key::compare`] function.
// FIXME:
// implement getting values using ranges.
// <https://github.com/Cuprate/cuprate/pull/117#discussion_r1589378104>
pub trait Key: Storable + Sized {
    /// Compare 2 [`Key`]'s against each other.
    ///
    /// # Defaults for types
    /// For arrays and vectors that contain a `T: Storable`,
    /// this does a straight _byte_ comparison, not a comparison of the key's value.
    ///
    /// For [`StorableStr`], this will use [`str::cmp`], i.e. it is the same as the default behavior; it is a
    /// [lexicographical comparison](https://doc.rust-lang.org/std/cmp/trait.Ord.html#lexicographical-comparison)
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
    /// let vec1 = StorableVec(vec![0, 1]);
    /// let vec2 = StorableVec(vec![255, 0]);
    /// assert_eq!(
    ///     <StorableVec<u8> as Key>::compare(&vec1, &vec2),
    ///     std::cmp::Ordering::Less,
    /// );
    ///
    /// // Integer comparison.
    /// let byte1 = [0, 1]; // 256
    /// let byte2 = [255, 0]; // 255
    /// let num1 = u16::from_le_bytes(byte1);
    /// let num2 = u16::from_le_bytes(byte2);
    /// assert_eq!(num1, 256);
    /// assert_eq!(num2, 255);
    /// assert_eq!(
    ///     //                    256 > 255
    ///     <u16 as Key>::compare(&byte1, &byte2),
    ///     std::cmp::Ordering::Greater,
    /// );
    /// ```
    #[inline]
    fn compare(left: &[u8], right: &[u8]) -> Ordering {
        left.cmp(right)
    }
}

//---------------------------------------------------------------------------------------------------- Impl
/// [`Ord`] comparison for arrays/vectors.
impl<const N: usize, T> Key for [T; N] where T: Key + Storable + Sized + bytemuck::Pod {}
impl<T: bytemuck::Pod + Debug> Key for StorableVec<T> {}

/// [`Ord`] comparison for any `T`.
///
/// This is not a blanket implementation because
/// it allows outer crates to define their own
/// comparison functions for their `T: Storable` types.
impl Key for () {}
impl Key for StorableBytes {}
impl Key for StorableStr {}

/// Integer comparison for numbers.
macro_rules! impl_key_ne_bytes {
    ($($t:ident),* $(,)?) => {
        $(
            impl Key for $t {
                #[inline]
                fn compare(left: &[u8], right: &[u8]) -> Ordering {
                    // INVARIANT:
                    // This is native endian since [`Storable`] (bytemuck, really)
                    // (de)serializes bytes with native endian.
                    let left = $t::from_ne_bytes(left.try_into().unwrap());
                    let right = $t::from_ne_bytes(right.try_into().unwrap());
                    std::cmp::Ord::cmp(&left, &right)
                }
            }
        )*
    };
}

impl_key_ne_bytes! {
    u8,u16,u32,u64,u128,usize,
    i8,i16,i32,i64,i128,isize,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

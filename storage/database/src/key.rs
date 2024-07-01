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
/// The database backends will use [`Key::KEY_COMPARE`]
/// to sort the keys within database tables.
///
/// [`Key::KEY_COMPARE`] is pre-implemented as a straight byte comparison.
///
/// This default is overridden for numbers, which use a number comparison.
/// For example, [`u64`] keys are sorted as `{0, 1, 2 ... 999_998, 999_999, 1_000_000}`.
///
/// If you would like to re-define this for number types, consider;
/// 1. Creating a wrapper type around primitives like a `struct SortU8(pub u8)`
/// 2. Implement [`Key`] on that wrapper
/// 3. Define a custom [`Key::KEY_COMPARE`]
// FIXME:
// implement getting values using ranges.
// <https://github.com/Cuprate/cuprate/pull/117#discussion_r1589378104>
pub trait Key: Storable + Sized + Ord {
    /// Compare 2 [`Key`]'s against each other.
    ///
    /// # Defaults for types
    /// For arrays and vectors that contain a `T: Storable`,
    /// this does a straight _byte_ comparison, not a comparison of the key's value.
    ///
    /// For [`StorableStr`], this will use [`str::cmp`], i.e. it is the same as the default behavior; it is a
    /// [lexicographical comparison](https://doc.rust-lang.org/std/cmp/trait.Ord.html#lexicographical-comparison)
    ///
    /// For all primitive number types ([`u8`], [`i128`], etc), this will
    /// convert the bytes to the number using [`Storable::from_bytes`],
    /// then do a number comparison.
    ///
    /// # Example
    /// ```rust
    /// # use cuprate_database::*;
    /// // Normal byte comparison.
    /// let vec1 = StorableVec(vec![0, 1]);
    /// let vec2 = StorableVec(vec![255, 0]);
    /// assert_eq!(
    ///     <StorableVec<u8> as Key>::KEY_COMPARE
    ///         .as_compare_fn::<StorableVec<u8>>()(&vec1, &vec2),
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
    ///     //                                               256 > 255
    ///     <u16 as Key>::KEY_COMPARE.as_compare_fn::<u16>()(&byte1, &byte2),
    ///     std::cmp::Ordering::Greater,
    /// );
    /// ```
    const KEY_COMPARE: KeyCompare = KeyCompare::Lexicographic;
}

//---------------------------------------------------------------------------------------------------- Impl
/// [`Ord`] comparison for arrays/vectors.
impl<const N: usize, T> Key for [T; N] where T: Key + Storable + Sized + bytemuck::Pod {}
impl<T: bytemuck::Pod + Debug + Ord> Key for StorableVec<T> {}

/// [`Ord`] comparison for misc types.
///
/// This is not a blanket implementation because
/// it allows outer crates to define their own
/// comparison functions for their `T: Storable` types.
impl Key for () {}
impl Key for StorableBytes {}
impl Key for StorableStr {}

/// Number comparison.
///
/// # Invariant
/// This must _only_ be implemented for [`u32`], [`u64`] (and maybe [`usize`]).
///
/// This is because:
/// 1. We use LMDB's `INTEGER_KEY` flag when this enum variant is used
/// 2. LMDB only supports these types when using that flag
///
/// See: <https://docs.rs/heed/0.20.0-alpha.9/heed/struct.DatabaseFlags.html#associatedconstant.INTEGER_KEY>
///
/// Other numbers will still have the same behavior, but they use
/// [`impl_custom_numbers_key`] and essentially pass LMDB a "custom"
/// number compare function.
macro_rules! impl_number_key {
    ($($t:ident),* $(,)?) => {
        $(
            impl Key for $t {
                const KEY_COMPARE: KeyCompare = KeyCompare::Number;
            }
        )*
    };
}

impl_number_key!(u32, u64, usize);
#[cfg(not(any(target_pointer_width = "32", target_pointer_width = "64")))]
compile_error!("`cuprate_database`: `usize` must be equal to `u32` or `u64` for LMDB's `usize` key sorting to function correctly");

/// Custom number comparison for other numbers.
macro_rules! impl_custom_numbers_key {
    ($($t:ident),* $(,)?) => {
        $(
            impl Key for $t {
                // Just forward the the number comparison function.
                const KEY_COMPARE: KeyCompare = KeyCompare::Custom(|left, right| {
                    KeyCompare::Number.as_compare_fn::<$t>()(left, right)
                });
            }
        )*
    };
}

impl_custom_numbers_key!(u8, u16, u128, i8, i16, i32, i64, i128, isize);

//---------------------------------------------------------------------------------------------------- KeyCompare
/// Comparison behavior for [`Key`]s.
///
/// This determines how the database sorts [`Key`]s inside a database [`Table`](crate::Table).
///
/// See [`Key`] for more info.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyCompare {
    /// [Lexicographical comparison](https://doc.rust-lang.org/1.79.0/std/cmp/trait.Ord.html#lexicographical-comparison),
    /// i.e. a straight byte comparison.
    ///
    /// This is the default.
    #[default]
    Lexicographic,

    /// A by-value number comparison, i.e. `255 < 256`.
    ///
    /// This _behavior_ is implemented as the default for all number primitives,
    /// although some implementations on numbers use [`KeyCompare::Custom`] due
    /// to internal implementation details of LMDB.
    Number,

    /// A custom sorting function.
    ///
    /// The input of the function is 2 [`Key`]s in byte form.
    Custom(fn(&[u8], &[u8]) -> Ordering),
}

impl KeyCompare {
    /// Return [`Self`] as a pure comparison function.
    ///
    /// This function's expects 2 [`Key`]s in byte form as input.
    #[inline]
    pub const fn as_compare_fn<K: Key>(self) -> fn(&[u8], &[u8]) -> Ordering {
        match self {
            Self::Lexicographic => std::cmp::Ord::cmp,
            Self::Number => |left, right| {
                let left = <K as Storable>::from_bytes(left);
                let right = <K as Storable>::from_bytes(right);
                std::cmp::Ord::cmp(&left, &right)
            },
            Self::Custom(f) => f,
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

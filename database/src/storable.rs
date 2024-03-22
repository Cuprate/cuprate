//! (De)serialization for table keys & values.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::{Borrow, Cow},
    char::ToLowercase,
    fmt::Debug,
    io::{Read, Write},
    sync::Arc,
};

use bytemuck::{Pod, Zeroable};

use crate::ToOwnedDebug;

//---------------------------------------------------------------------------------------------------- Storable
/// A type that can be stored in the database.
///
/// All keys and values in the database must be able
/// to be (de)serialized into/from raw bytes (`[u8]`).
///
/// This trait represents types that can be **perfectly**
/// casted/represented as raw bytes.
///
/// ## `bytemuck`
/// Any type that implements:
/// - [`bytemuck::Pod`]
/// - [`Debug`]
/// - [`ToOwned`]
///
/// will automatically implement [`Storable`].
///
/// This includes:
/// - Most primitive types
/// - All types in [`tables`](crate::tables)
/// - Slices, e.g, `[T] where T: Storable`
///
/// See [`StorableVec`] for storing slices of `T: Storable`.
///
/// ```rust
/// # use cuprate_database::*;
/// # use std::borrow::*;
/// let number: u64 = 0;
///
/// // Into bytes.
/// let into = Storable::as_bytes(&number);
/// assert_eq!(into, &[0; 8]);
///
/// // From bytes.
/// let from: u64 = Storable::from_bytes(&into);
/// assert_eq!(from, number);
/// ```
///
/// ## Invariants
/// No function in this trait is expected to panic.
///
/// The byte conversions must execute flawlessly.
///
/// ## Endianness
/// This trait doesn't currently care about endianness.
///
/// Bytes are (de)serialized as-is, and `bytemuck`
/// types are architecture-dependant.
///
/// Most likely, the bytes are little-endian, however
/// that cannot be relied upon when using this trait.
pub trait Storable: ToOwnedDebug {
    /// Is this type fixed width in byte length?
    ///
    /// I.e., when converting `Self` to bytes, is it
    /// represented with a fixed length array of bytes?
    ///
    /// # `Some`
    /// This should be `Some(usize)` on types like:
    /// - `u8`
    /// - `u64`
    /// - `i32`
    ///
    /// where the byte length is known.
    ///
    /// # `None`
    /// This should be `None` on any variable-length type like:
    /// - `str`
    /// - `[u8]`
    /// - `Vec<u8>`
    ///
    /// # Examples
    /// ```rust
    /// # use cuprate_database::*;
    /// assert_eq!(<()>::BYTE_LENGTH, Some(0));
    /// assert_eq!(u8::BYTE_LENGTH, Some(1));
    /// assert_eq!(u16::BYTE_LENGTH, Some(2));
    /// assert_eq!(u32::BYTE_LENGTH, Some(4));
    /// assert_eq!(u64::BYTE_LENGTH, Some(8));
    /// assert_eq!(i8::BYTE_LENGTH, Some(1));
    /// assert_eq!(i16::BYTE_LENGTH, Some(2));
    /// assert_eq!(i32::BYTE_LENGTH, Some(4));
    /// assert_eq!(i64::BYTE_LENGTH, Some(8));
    /// assert_eq!(StorableVec::<u8>::BYTE_LENGTH, None);
    /// assert_eq!(StorableVec::<u64>::BYTE_LENGTH, None);
    /// ```
    const BYTE_LENGTH: Option<usize>;

    /// Return `self` in byte form.
    fn as_bytes(&self) -> &[u8];

    /// Create an owned [`Self`] from bytes.
    ///
    /// # Blanket implementation
    /// The blanket implementation that covers all types used
    /// by `cuprate_database` will simply bitwise copy `bytes`
    /// into `Self`.
    ///
    /// The bytes do not have be correctly aligned.
    fn from_bytes(bytes: &[u8]) -> Self;
}

impl<T> Storable for T
where
    Self: Pod + ToOwnedDebug<OwnedDebug = T>,
{
    const BYTE_LENGTH: Option<usize> = Some(std::mem::size_of::<T>());

    #[inline]
    fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> T {
        bytemuck::pod_read_unaligned(bytes)
    }
}

//---------------------------------------------------------------------------------------------------- StorableVec
/// A [`Storable`] vector of `T: Storable`.
///
/// This is a wrapper around `Vec<T> where T: Storable`.
///
/// Slice types are owned both:
/// - when returned from the database
/// - in `put()`
///
/// This is needed as `impl Storable for Vec<T>` runs into impl conflicts.
///
/// ```rust
/// # use cuprate_database::*;
/// //---------------------------------------------------- u8
/// let vec: StorableVec<u8> = StorableVec(vec![0,1]);
///
/// // Into bytes.
/// let into = Storable::as_bytes(&vec);
/// assert_eq!(into, &[0,1]);
///
/// // From bytes.
/// let from: StorableVec<u8> = Storable::from_bytes(&into);
/// assert_eq!(from, vec);
///
/// //---------------------------------------------------- u64
/// let vec: StorableVec<u64> = StorableVec(vec![0,1]);
///
/// // Into bytes.
/// let into = Storable::as_bytes(&vec);
/// assert_eq!(into, &[0,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0]);
///
/// // From bytes.
/// let from: StorableVec<u64> = Storable::from_bytes(&into);
/// assert_eq!(from, vec);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, bytemuck::TransparentWrapper)]
#[repr(transparent)]
pub struct StorableVec<T>(pub Vec<T>);

impl<T> Storable for StorableVec<T>
where
    T: Pod + ToOwnedDebug<OwnedDebug = T>,
{
    const BYTE_LENGTH: Option<usize> = None;

    /// Casts the inner `Vec<T>` directly as bytes.
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        bytemuck::must_cast_slice(&self.0)
    }

    /// This always allocates a new `Vec<T>`,
    /// casting `bytes` into a vector of type `T`.
    #[inline]
    fn from_bytes(bytes: &[u8]) -> Self {
        Self(bytemuck::pod_collect_to_vec(bytes))
    }
}

impl<T> std::ops::Deref for StorableVec<T> {
    type Target = [T];
    #[inline]
    fn deref(&self) -> &[T] {
        &self.0
    }
}

impl<T> Borrow<[T]> for StorableVec<T> {
    #[inline]
    fn borrow(&self) -> &[T] {
        &self.0
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use super::*;

    /// Serialize, deserialize, and compare that
    /// the intermediate/end results are correct.
    fn test_storable<const LEN: usize, T>(
        // The primitive number function that
        // converts the number into little endian bytes,
        // e.g `u8::to_le_bytes`.
        to_le_bytes: fn(T) -> [u8; LEN],
        // A `Vec` of the numbers to test.
        t: Vec<T>,
    ) where
        T: Storable + ToOwnedDebug<OwnedDebug = T> + Copy + PartialEq,
    {
        for t in t {
            let expected_bytes = to_le_bytes(t);

            println!("testing: {t:?}, expected_bytes: {expected_bytes:?}");

            // (De)serialize.
            let se: &[u8] = Storable::as_bytes(&t);
            let de = <T as Storable>::from_bytes(se);

            println!("serialized: {se:?}, deserialized: {de:?}\n");

            // Assert we wrote correct amount of bytes.
            if let Some(len) = T::BYTE_LENGTH {
                assert_eq!(se.len(), expected_bytes.len());
            }
            // Assert the data is the same.
            assert_eq!(de, t);
        }
    }

    /// Create all the float tests.
    macro_rules! test_float {
        ($(
            $float:ident // The float type.
        ),* $(,)?) => {
            $(
                #[test]
                fn $float() {
                    test_storable(
                        $float::to_le_bytes,
                        vec![
                            -1.0,
                            0.0,
                            1.0,
                            $float::MIN,
                            $float::MAX,
                            $float::INFINITY,
                            $float::NEG_INFINITY,
                        ],
                    );
                }
            )*
        };
    }

    test_float! {
        f32,
        f64,
    }

    /// Create all the (un)signed number tests.
    /// u8 -> u128, i8 -> i128.
    macro_rules! test_unsigned {
        ($(
            $number:ident // The integer type.
        ),* $(,)?) => {
            $(
                #[test]
                fn $number() {
                    test_storable($number::to_le_bytes, vec![$number::MIN, 0, 1, $number::MAX]);
                }
            )*
        };
    }

    test_unsigned! {
        u8,
        u16,
        u32,
        u64,
        u128,
        usize,
        i8,
        i16,
        i32,
        i64,
        i128,
        isize,
    }
}

//! (De)serialization for table keys & values.

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::Cow,
    fmt::Debug,
    io::{Read, Write},
    sync::Arc,
};

use bytemuck::{AnyBitPattern, NoUninit};

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
/// Any type that implements `bytemuck`'s [`NoUninit`] + [`AnyBitPattern`]
/// (and [Debug]) will automatically implement [`Storable`].
///
/// This includes:
/// - Most primitive types
/// - All types in [`tables`](crate::tables)
/// - Slices, e.g, `[T] where T: Storable`
///
/// ```rust
/// # use cuprate_database::*;
/// let number: u64 = 0;
///
/// // Into bytes.
/// let into = Storable::as_bytes(&number);
/// assert_eq!(into, &[0; 8]);
///
/// // From bytes.
/// let from: &u64 = Storable::from_bytes(&into);
/// assert_eq!(from, &number);
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
pub trait Storable: Debug {
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
    /// # use cuprate_database::Storable;
    /// assert_eq!(<()>::BYTE_LENGTH, Some(0));
    /// assert_eq!(u8::BYTE_LENGTH, Some(1));
    /// assert_eq!(u16::BYTE_LENGTH, Some(2));
    /// assert_eq!(u32::BYTE_LENGTH, Some(4));
    /// assert_eq!(u64::BYTE_LENGTH, Some(8));
    /// assert_eq!(i8::BYTE_LENGTH, Some(1));
    /// assert_eq!(i16::BYTE_LENGTH, Some(2));
    /// assert_eq!(i32::BYTE_LENGTH, Some(4));
    /// assert_eq!(i64::BYTE_LENGTH, Some(8));
    /// assert_eq!(<[u8]>::BYTE_LENGTH, None);
    /// assert_eq!(<[u8; 0]>::BYTE_LENGTH, Some(0));
    /// assert_eq!(<[u8; 1]>::BYTE_LENGTH, Some(1));
    /// assert_eq!(<[u8; 2]>::BYTE_LENGTH, Some(2));
    /// assert_eq!(<[u8; 3]>::BYTE_LENGTH, Some(3));
    /// ```
    const BYTE_LENGTH: Option<usize>;

    /// Return `self` in byte form.
    fn as_bytes(&self) -> &[u8];

    /// Create [`Self`] from bytes.
    fn from_bytes(bytes: &[u8]) -> &Self;
}

//---------------------------------------------------------------------------------------------------- Impl
impl<T: NoUninit + AnyBitPattern + Debug> Storable for T {
    const BYTE_LENGTH: Option<usize> = Some(std::mem::size_of::<T>());

    #[inline]
    fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> &Self {
        bytemuck::from_bytes(bytes)
    }
}

impl<T: NoUninit + AnyBitPattern + Debug> Storable for [T] {
    const BYTE_LENGTH: Option<usize> = None;

    #[inline]
    fn as_bytes(&self) -> &[u8] {
        bytemuck::must_cast_slice(self)
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> &Self {
        bytemuck::must_cast_slice(bytes)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use super::*;

    /// Serialize, deserialize, and compare that
    /// the intermediate/end results are correct.
    fn test_storable<const LEN: usize, T: Storable + Copy + PartialEq>(
        // The primitive number function that
        // converts the number into little endian bytes,
        // e.g `u8::to_le_bytes`.
        to_le_bytes: fn(T) -> [u8; LEN],
        // A `Vec` of the numbers to test.
        t: Vec<T>,
    ) {
        for t in t {
            let expected_bytes = to_le_bytes(t);

            println!("testing: {t:?}, expected_bytes: {expected_bytes:?}");

            // (De)serialize.
            let se: &[u8] = Storable::as_bytes(&t);
            let de: &T = Storable::from_bytes(se);

            println!("serialized: {se:?}, deserialized: {de:?}\n");

            // Assert we wrote correct amount of bytes.
            if let Some(len) = T::BYTE_LENGTH {
                assert_eq!(se.len(), expected_bytes.len());
            }
            // Assert the data is the same.
            assert_eq!(de, &t);
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

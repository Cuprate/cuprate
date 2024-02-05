//! (De)serialization for table keys & values.
//!
//! All keys and values in database tables must be able
//! to be (de)serialized into/from raw bytes ([u8]).

//---------------------------------------------------------------------------------------------------- Import
use std::{
    borrow::Cow,
    io::{Read, Write},
    sync::Arc,
};

//---------------------------------------------------------------------------------------------------- Pod
/// Plain Old Data.
///
/// Trait representing very simple types that can be
/// (de)serialized into/from bytes.
///
/// Reference: <https://docs.rs/bytemuck/latest/bytemuck/trait.Pod.html>
///
/// ## Endianness
/// As `bytemuck` provides everything needed here + more, it could be used,
/// _but_, its `Pod` is endian dependant. We need to ensure bytes are the
/// exact same such that the database stores the same bytes on different machines;
/// so we use little endian functions instead, e.g. [`u8::to_le_bytes`].
///
/// This also means an `INVARIANT` of this trait is that
/// implementors must use little endian bytes when applicable.
///
/// Slice types (just raw `[u8]` bytes) are (de)serialized as-is.
///
/// ## Sealed
/// This trait is [`Sealed`](https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed).
///
/// It cannot be implemented outside this crate,
/// and is only implemented on specific types.
pub trait Pod: Sized + private::Sealed {
    /// Return `self` in byte form.
    ///
    /// The returned bytes can be any form of array,
    /// - [`[u8; N]`]
    /// - [`Vec<u8>`]
    /// - [`&[u8]`]
    ///
    /// ..etc.
    ///
    /// This is used on slice types (`Vec<u8>`, `[u8; N]`, etc) for cheap conversions.
    ///
    /// Integer types ([`u8`], [`f32`], [`i8`], etc) return a fixed-sized array.
    fn as_bytes(&self) -> impl AsRef<[u8]>;

    /// TODO
    fn into_bytes(self) -> Cow<'static, [u8]>;

    /// Create [`Self`] from bytes.
    ///
    /// # Errors
    /// If:
    /// 1. `Self` is an exact sized array, e.g `[u8; 4]` AND
    /// 2. `bytes`'s length is not exactly that length then
    ///
    /// this function will return `Err(usize)`,
    /// returning the length of `bytes`
    ///
    /// In the case of `Vec<u8>` and `Box<[u8]>`, this function
    /// will never fail, and always return [`Ok`].
    fn from_bytes(bytes: &[u8]) -> Result<Self, usize>;

    /// Convert [`Self`] into bytes, and write those bytes into a [`Write`]r.
    ///
    /// # Errors
    /// This only returns an error if the `writer` itself errors for some reason.
    ///
    /// That error is forwarded, else, the amount of bytes written is returned in [`Ok`].
    fn to_writer<W: Write>(self, writer: &mut W) -> std::io::Result<usize>;

    /// Create [`Self`] by reading bytes from a [`Read`]er.
    ///
    /// # Errors
    /// This only returns an error if the `reader` itself errors for some reason.
    ///
    /// That error is forwarded, else, [`Self`] is returned in [`Ok`].
    fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self>;
}

/// Private module, should not be accessible outside this crate.
///
/// Used to block outsiders implementing [`Pod`].
/// All [`Pod`] types must also implement [`Sealed`].
mod private {
    /// Private sealed trait.
    ///
    /// Cannot be implemented outside this crate.
    pub trait Sealed {}

    /// Implement `Sealed`.
    macro_rules! impl_sealed {
        ($(
            $t:ty // The type to implement for.
        ),* $(,)?) => {
            $(
                impl Sealed for $t {}
            )*
        };
    }

    // Special case cause of generic.
    impl<const N: usize> Sealed for [u8; N] {}

    impl_sealed! {
        Vec<u8>,
        Box<[u8]>,
        std::sync::Arc<[u8]>,
        f32,
        f64,
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

//---------------------------------------------------------------------------------------------------- Pod Impl (bytes)
// Implement for owned `Vec` bytes.
impl Pod for Vec<u8> {
    fn as_bytes(&self) -> impl AsRef<[u8]> {
        self
    }

    fn into_bytes(self) -> Cow<'static, [u8]> {
        Cow::Owned(self)
    }

    /// This function will always return [`Ok`].
    fn from_bytes(bytes: &[u8]) -> Result<Self, usize> {
        Ok(bytes.to_vec())
    }

    fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        // FIXME: Could be `Vec::with_capacity(likely_size)`?
        let mut vec = vec![];

        reader.read_to_end(&mut vec)?;

        Ok(vec)
    }

    fn to_writer<W: Write>(self, writer: &mut W) -> std::io::Result<usize> {
        writer.write_all(&self)?;
        Ok(self.len())
    }
}

// Implement for any sized stack array.
impl<const N: usize> Pod for [u8; N] {
    fn as_bytes(&self) -> impl AsRef<[u8]> {
        self
    }

    fn into_bytes(self) -> Cow<'static, [u8]> {
        Cow::Owned(self.to_vec())
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, usize> {
        // Return if the bytes are too short/long.
        let bytes_len = bytes.len();
        if bytes_len != N {
            return Err(bytes_len);
        }

        let mut array = [0_u8; N];
        // INVARIANT: we checked the length is valid above.
        array.copy_from_slice(bytes);

        Ok(array)
    }

    fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut bytes = [0_u8; N];
        reader.read_exact(&mut bytes)?;
        Ok(bytes)
    }

    fn to_writer<W: Write>(self, writer: &mut W) -> std::io::Result<usize> {
        writer.write_all(&self)?;
        Ok(self.len())
    }
}

// Implement for any sized boxed array.
//
// In-case `[u8; N]` is too big and would
// overflow the stack, this can be used.
//
// The benefit over `Vec<u8>` is that the capacity & length are static.
//
// The weird constructions of `Box` below are on purpose to avoid this:
// <https://github.com/rust-lang/rust/issues/53827>
impl Pod for Box<[u8]> {
    fn as_bytes(&self) -> impl AsRef<[u8]> {
        self
    }

    fn into_bytes(self) -> Cow<'static, [u8]> {
        Cow::Owned(self.into())
    }

    /// This function will always return [`Ok`].
    fn from_bytes(bytes: &[u8]) -> Result<Self, usize> {
        Ok(Self::from(bytes))
    }

    fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut bytes = vec![];
        reader.read_to_end(bytes.as_mut())?;
        Ok(bytes.into_boxed_slice())
    }

    fn to_writer<W: Write>(self, writer: &mut W) -> std::io::Result<usize> {
        writer.write_all(&self)?;
        Ok(self.len())
    }
}

// Implement for any Arc bytes.
impl Pod for Arc<[u8]> {
    fn as_bytes(&self) -> impl AsRef<[u8]> {
        self
    }

    fn into_bytes(self) -> Cow<'static, [u8]> {
        Cow::Owned(self.to_vec())
    }

    /// This function will always return [`Ok`].
    fn from_bytes(bytes: &[u8]) -> Result<Self, usize> {
        Ok(Self::from(bytes))
    }

    fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut bytes = vec![];
        reader.read_to_end(bytes.as_mut())?;
        Ok(Self::from(bytes))
    }

    fn to_writer<W: Write>(self, writer: &mut W) -> std::io::Result<usize> {
        writer.write_all(&self)?;
        Ok(self.len())
    }
}

//---------------------------------------------------------------------------------------------------- Pod Impl (numbers)
/// Implement `Pod` on primitive numbers.
///
/// This will always use little endian representations.
macro_rules! impl_pod_le_bytes {
    ($(
        $number:ident => // The number type.
        $length:literal  // The length of `u8`'s this type takes up.
    ),* $(,)?) => {
        $(
            impl Pod for $number {
                fn as_bytes(&self) -> impl AsRef<[u8]> {
                    $number::to_le_bytes(*self)
                }

                fn into_bytes(self) -> Cow<'static, [u8]> {
                    Cow::Owned(self.as_bytes().as_ref().to_vec())
                }

                /// This function returns [`Err`] if `bytes`'s length is not
                #[doc = concat!(" ", stringify!($length), ".")]
                fn from_bytes(bytes: &[u8]) -> Result<Self, usize> {
                    // Return if the bytes are too short/long.
                    let bytes_len = bytes.len();
                    if bytes_len != $length {
                        return Err(bytes_len);
                    }

                    let mut array = [0_u8; $length];
                    // INVARIANT: we checked the length is valid above.
                    array.copy_from_slice(bytes);

                    Ok($number::from_le_bytes(array))
                }

                fn to_writer<W: Write>(self, writer: &mut W) -> std::io::Result<usize> {
                    writer.write(self.as_bytes().as_ref())
                }

                fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
                    let mut bytes = [0_u8; $length];

                    // Read exactly the bytes required.
                    reader.read_exact(&mut bytes)?;

                    // INVARIANT: we checked the length is valid above.
                    Ok($number::from_le_bytes(bytes))
                }
            }
        )*
    };
}

impl_pod_le_bytes! {
    f32   => 4,
    f64   => 8,

    u8    => 1,
    u16   => 2,
    u32   => 4,
    u64   => 8,
    usize => 8,
    u128  => 16,

    i8    => 1,
    i16   => 2,
    i32   => 4,
    i64   => 8,
    isize => 8,
    i128  => 16,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use super::*;

    /// Serialize, deserialize, and compare that
    /// the intermediate/end results are correct.
    fn test_serde<const LEN: usize, T: Pod + Copy + PartialEq + std::fmt::Debug>(
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

            let mut bytes = vec![];

            // (De)serialize.
            let se: usize = t.to_writer::<Vec<u8>>(bytes.as_mut()).unwrap();
            let de: T = T::from_reader::<&[u8]>(&mut bytes.as_slice()).unwrap();

            println!("written: {se}, deserialize_t: {de:?}, bytes: {bytes:?}\n");

            // Assert we wrote correct amount of bytes
            // and deserialized correctly.
            assert_eq!(se, expected_bytes.len());
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
                    test_serde(
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
                    test_serde($number::to_le_bytes, vec![$number::MIN, 0, 1, $number::MAX]);
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

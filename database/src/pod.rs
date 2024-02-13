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
///
/// # TODO
/// This could be implemented on `bytes::Bytes` if needed.
///
/// Maybe under a `bytes` feature flag.
pub trait Pod: Sized + private::Sealed {
    /// Return `self` in byte form.
    ///
    /// The returned bytes can be any form of array,
    /// - `[u8]`
    /// - `[u8; N]`
    /// - [`Vec<u8>`]
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
    /// # Panics
    /// This function should be infallible.
    ///
    /// If `bytes` is invalid, this should panic.
    fn from_bytes(bytes: &[u8]) -> Self;

    /// Convert [`Self`] into bytes, and write those bytes into a [`Write`]r.
    ///
    /// The `usize` returned should be how many bytes were written.
    ///
    /// TODO: do we ever actually need how many bytes were written?
    ///
    /// # Panics
    /// This function should be infallible.
    ///
    /// If the `writer` errors, this should panic.
    fn to_writer<W: Write>(self, writer: &mut W) -> usize;

    /// Create [`Self`] by reading bytes from a [`Read`]er.
    ///
    /// # Panics
    /// This function should be infallible.
    ///
    /// If the `reader` errors, this should panic.
    fn from_reader<R: Read>(reader: &mut R) -> Self;
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
    // [`crate::key::DupKey`]
    impl<P: Sealed, S: Sealed> Sealed for crate::key::DupKey<P, S> {}

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
    #[inline]
    fn as_bytes(&self) -> impl AsRef<[u8]> {
        self
    }

    #[inline]
    fn into_bytes(self) -> Cow<'static, [u8]> {
        Cow::Owned(self)
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> Self {
        bytes.to_vec()
    }

    #[inline]
    fn from_reader<R: Read>(reader: &mut R) -> Self {
        // FIXME: Could be `Vec::with_capacity(likely_size)`?
        let mut vec = vec![];

        reader
            .read_to_end(&mut vec)
            .expect("Pod::<Vec<u8>>::read_to_end() failed");

        vec
    }

    #[inline]
    fn to_writer<W: Write>(self, writer: &mut W) -> usize {
        writer
            .write_all(&self)
            .expect("Pod::<Vec<u8>>::write_all() failed");

        self.len()
    }
}

// Implement for any sized stack array.
impl<const N: usize> Pod for [u8; N] {
    #[inline]
    fn as_bytes(&self) -> impl AsRef<[u8]> {
        self
    }

    #[inline]
    fn into_bytes(self) -> Cow<'static, [u8]> {
        Cow::Owned(self.to_vec())
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> Self {
        // Return if the bytes are too short/long.
        let bytes_len = bytes.len();
        assert_eq!(
            bytes_len, N,
            "Pod::<[u8; {N}]>::from_bytes() failed, expected_len: {N}, found_len: {bytes_len}",
        );

        let mut array = [0_u8; N];
        // INVARIANT: we checked the length is valid above.
        array.copy_from_slice(bytes);

        array
    }

    #[inline]
    fn from_reader<R: Read>(reader: &mut R) -> Self {
        let mut bytes = [0_u8; N];
        reader
            .read_exact(&mut bytes)
            .expect("Pod::<[u8; {N}]>::read_exact() failed");

        bytes
    }

    #[inline]
    fn to_writer<W: Write>(self, writer: &mut W) -> usize {
        writer
            .write_all(&self)
            .expect("Pod::<[u8; {N}]>::write_all() failed");
        self.len()
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
    #[inline]
    fn as_bytes(&self) -> impl AsRef<[u8]> {
        self
    }

    #[inline]
    fn into_bytes(self) -> Cow<'static, [u8]> {
        Cow::Owned(self.into())
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> Self {
        Self::from(bytes)
    }

    #[inline]
    fn from_reader<R: Read>(reader: &mut R) -> Self {
        let mut bytes = vec![];
        reader
            .read_to_end(bytes.as_mut())
            .expect("Pod::<Box<[u8]>>::read_to_end() failed");
        bytes.into_boxed_slice()
    }

    #[inline]
    fn to_writer<W: Write>(self, writer: &mut W) -> usize {
        writer
            .write_all(&self)
            .expect("Pod::<Box<[u8]>>::write_all() failed");
        self.len()
    }
}

// Implement for any Arc bytes.
impl Pod for Arc<[u8]> {
    #[inline]
    fn as_bytes(&self) -> impl AsRef<[u8]> {
        self
    }

    #[inline]
    fn into_bytes(self) -> Cow<'static, [u8]> {
        Cow::Owned(self.to_vec())
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> Self {
        Self::from(bytes)
    }

    #[inline]
    fn from_reader<R: Read>(reader: &mut R) -> Self {
        let mut bytes = vec![];
        reader
            .read_to_end(bytes.as_mut())
            .expect("Pod::<Arc<[u8]>>::read_to_end() failed");
        Self::from(bytes)
    }

    #[inline]
    fn to_writer<W: Write>(self, writer: &mut W) -> usize {
        writer
            .write_all(&self)
            .expect("Pod::<Arc<[u8]>>::write_all() failed");
        self.len()
    }
}

// Implement for any `DupKey` that has types that implement `Pod`.
//
// TODO: how to serialize this?
impl<P, S> Pod for crate::key::DupKey<P, S>
where
    P: Pod,
    S: Pod,
{
    #[inline]
    fn as_bytes(&self) -> impl AsRef<[u8]> {
        let primary: &[u8] = self.primary.as_bytes().as_ref();
        let secondary: &[u8] = self.secondary.as_bytes().as_ref();

        // TODO: trait bound fails?
        // primary.concat(secondary)

        let bytes: &[u8] = todo!();
        bytes
    }

    #[inline]
    fn into_bytes(self) -> Cow<'static, [u8]> {
        Cow::Owned(self.as_bytes().as_ref().to_vec())
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> Self {
        let primary = P::from_bytes(bytes);

        // TODO: split `bytes` by size of `P, S`'s
        // byte lengths and deserialize...?

        todo!();
    }

    #[inline]
    fn from_reader<R: Read>(reader: &mut R) -> Self {
        let mut bytes = vec![];
        reader
            .read_to_end(bytes.as_mut())
            .expect("Pod::<Arc<[u8]>>::read_to_end() failed");

        Self::from_bytes(&bytes)
    }

    #[inline]
    fn to_writer<W: Write>(self, writer: &mut W) -> usize {
        // TODO: serialize both primary and secondary?

        self.primary.to_writer(writer);
        self.secondary.to_writer(writer);

        // TODO: Return length?
        todo!()
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
                #[inline]
                fn as_bytes(&self) -> impl AsRef<[u8]> {
                    $number::to_le_bytes(*self)
                }

                #[inline]
                fn into_bytes(self) -> Cow<'static, [u8]> {
                    Cow::Owned(self.as_bytes().as_ref().to_vec())
                }

                #[inline]
                /// This function returns [`Err`] if `bytes`'s length is not
                #[doc = concat!(" ", stringify!($length), ".")]
                fn from_bytes(bytes: &[u8]) -> Self {
                    // Return if the bytes are too short/long.
                    let bytes_len = bytes.len();
                    assert_eq!(
                        bytes_len, $length,
                        "Pod::<[u8; {0}]>::from_bytes() failed, expected_len: {0}, found_len: {bytes_len}",
                        $length,
                    );

                    let mut array = [0_u8; $length];
                    // INVARIANT: we checked the length is valid above.
                    array.copy_from_slice(bytes);

                    $number::from_le_bytes(array)
                }

                #[inline]
                fn to_writer<W: Write>(self, writer: &mut W) -> usize {
                    writer.write_all(self.as_bytes().as_ref()).expect(concat!(
                        "Pod::<",
                        stringify!($number),
                        ">::write_all() failed",
                    ));
                    $length
                }

                #[inline]
                fn from_reader<R: Read>(reader: &mut R) -> Self {
                    let mut bytes = [0_u8; $length];

                    // Read exactly the bytes required.
                    reader.read_exact(&mut bytes).expect(concat!(
                        "Pod::<",
                        stringify!($number),
                        ">::react_exact() failed",
                    ));

                    // INVARIANT: we checked the length is valid above.
                    $number::from_le_bytes(bytes)
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
            let se: usize = t.to_writer::<Vec<u8>>(bytes.as_mut());
            let de: T = T::from_reader::<&[u8]>(&mut bytes.as_slice());

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

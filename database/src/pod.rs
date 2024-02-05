//! (De)serialization for table keys & values.
//!
//! All keys and values in database tables must be able
//! to be (de)serialized into/from raw bytes ([u8]).

//---------------------------------------------------------------------------------------------------- Import
// use crate::error::Error;

use std::io::{Read, Write};

//---------------------------------------------------------------------------------------------------- Pod
/// TODO
///
/// [P]lain [O]ld [D]ata.
///
/// Trait representing very simple types that can be
/// (de)serialized into/from bytes.
///
/// INVARIANT: little endian representations only?
///
/// reference: <https://docs.rs/bytemuck/latest/bytemuck/trait.Pod.html>
pub trait Pod: Sized {
    /// TODO
    /// # Errors
    /// TODO
    fn to_bytes<W: Write>(self, writer: &mut W) -> std::io::Result<usize>;

    /// TODO
    /// # Errors
    /// TODO
    fn from_bytes<R: Read>(reader: &mut R) -> std::io::Result<Self>;
}

//---------------------------------------------------------------------------------------------------- Pod Impl (bytes)
// Implement for owned `Vec` bytes.
impl Pod for Vec<u8> {
    fn from_bytes<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        // FIXME: Could be `Vec::with_capacity(likely_size)`?
        let mut vec = vec![];

        reader.read_to_end(&mut vec)?;

        Ok(vec)
    }

    fn to_bytes<W: Write>(self, writer: &mut W) -> std::io::Result<usize> {
        writer.write_all(&self)?;
        Ok(self.len())
    }
}

// Implement for any sized stack array.
impl<const N: usize> Pod for [u8; N] {
    fn from_bytes<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut bytes = [0_u8; N];
        reader.read_exact(&mut bytes)?;
        Ok(bytes)
    }

    fn to_bytes<W: Write>(self, writer: &mut W) -> std::io::Result<usize> {
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
                fn to_bytes<W: Write>(self, writer: &mut W) -> std::io::Result<usize> {
                    let bytes = $number::to_le_bytes(self);
                    writer.write(&bytes)
                }

                fn from_bytes<R: Read>(reader: &mut R) -> std::io::Result<Self> {
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
            let se: usize = t.to_bytes::<Vec<u8>>(bytes.as_mut()).unwrap();
            let de: T = T::from_bytes::<&[u8]>(&mut bytes.as_slice()).unwrap();

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

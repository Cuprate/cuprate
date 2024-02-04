//! (De)serialization for table keys & values.
//!
//! All keys and values in database tables must be able
//! to be (de)serialized into/from raw bytes ([u8]).

//---------------------------------------------------------------------------------------------------- Import
// use crate::error::Error;

use std::borrow::Cow;

//---------------------------------------------------------------------------------------------------- Constants

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
/// reference: <https://docs.rs/bytemuck>
pub trait Pod<const LEN: usize>: Sized {
    /// TODO
    ///
    /// FIXME: if we're only used a fixed-sized type
    /// we can get rid of `&` and just return something
    /// like [u8; 8].
    fn to_bytes(self) -> [u8; LEN];

    /// TODO
    /// # Errors
    /// TODO
    fn from_bytes(bytes: &[u8]) -> Result<Self, PodError>;
}

//---------------------------------------------------------------------------------------------------- Pod Impl
/// Implement `Pod` on primitive numbers.
///
/// This will always use little endian representations.
macro_rules! impl_pod_le_bytes {
    ($(
        $number:ident => // The number type.
        $length:literal  // The length of `u8`'s this type takes up.
    ),* $(,)?) => {
        $(
            impl Pod<$length> for $number {
                /// TODO
                fn to_bytes(self) -> [u8; $length] {
                    $number::to_le_bytes(self)
                }

                /// TODO
                /// # Errors
                /// TODO
                fn from_bytes(bytes: &[u8]) -> Result<Self, PodError> {
                    // Check for invalid length.
                    if bytes.len() != $length {
                        return Err(PodError::Length {
                            expected: $length,
                            found: bytes.len(),
                        });
                    }

                    // INVARIANT: we checked the length is valid above.
                    let bytes: [u8; $length] = bytes.try_into().unwrap();
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

//---------------------------------------------------------------------------------------------------- PodError
/// TODO
#[derive(thiserror::Error, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PodError {
    #[error("wrong length: expected `{expected}`, found `{found}`")]
    /// The byte length was incorrect.
    Length {
        /// The expected length of the `u8` representation.
        expected: usize,
        /// The incorrect found length.
        found: usize,
    },

    #[error("unknown error: {0}")]
    /// An unknown error occured.
    Unknown(Cow<'static, str>),
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    use super::*;

    /// Serialize, deserialize, and compare that
    /// the intermediate/end results are correct.
    fn test_serde<T: Pod<LEN>>(t: T, expected_bytes: &[u8]) {
        let se = t.to_bytes();
        let de = T::from_bytes(&se).unwrap();
        assert_eq!(se, expected_bytes);
        assert_eq!(de, t);
    }

    /// Test floats (de)serialize correctly.
    #[test]
    #[allow(clippy::float_cmp)]
    fn floats() {
        let f = 0.0_f32;
        let ser = f.to_bytes();
        let de = f32::from_bytes(&ser).unwrap();
        assert_eq!(ser, [0, 0, 0, 0]);
        assert_eq!(de, 0.0);

        let f = 1.0_f32;
        let ser = f.to_bytes();
        let de = f32::from_bytes(&ser).unwrap();
        assert_eq!(ser, [0, 0, 128, 63]);
        assert_eq!(de, 0.0);
    }
}

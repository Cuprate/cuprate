#![doc = include_str!("../README.md")]

use core::{
    fmt::{Debug, Formatter},
    ops::{Deref, Index},
};

use bytes::{BufMut, Bytes, BytesMut};

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize};

#[cfg_attr(feature = "std", derive(thiserror::Error))]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum FixedByteError {
    #[cfg_attr(
        feature = "std",
        error("Cannot create fix byte array, input has invalid length.")
    )]
    InvalidLength,
}

impl FixedByteError {
    fn field_name(&self) -> &'static str {
        match self {
            FixedByteError::InvalidLength => "input",
        }
    }

    fn field_data(&self) -> &'static str {
        match self {
            FixedByteError::InvalidLength => {
                "Cannot create fix byte array, input has invalid length."
            }
        }
    }
}

impl Debug for FixedByteError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FixedByteError")
            .field(self.field_name(), &self.field_data())
            .finish()
    }
}

/// A fixed size byte slice.
///
/// Internally this is just a wrapper around [`Bytes`], with the constructors checking that the length is equal to `N`.
/// This implements [`Deref`] with the target being `[u8; N]`.
#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[repr(transparent)]
pub struct ByteArray<const N: usize>(Bytes);

#[cfg(feature = "serde")]
impl<'de, const N: usize> Deserialize<'de> for ByteArray<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = Bytes::deserialize(deserializer)?;
        let len = bytes.len();
        if len == N {
            Ok(Self(bytes))
        } else {
            Err(serde::de::Error::invalid_length(
                len,
                &N.to_string().as_str(),
            ))
        }
    }
}

impl<const N: usize> ByteArray<N> {
    pub fn take_bytes(self) -> Bytes {
        self.0
    }
}

impl<const N: usize> From<[u8; N]> for ByteArray<N> {
    fn from(value: [u8; N]) -> Self {
        ByteArray(Bytes::copy_from_slice(&value))
    }
}

impl<const N: usize> Deref for ByteArray<N> {
    type Target = [u8; N];

    fn deref(&self) -> &Self::Target {
        self.0.deref().try_into().unwrap()
    }
}

impl<const N: usize> TryFrom<Bytes> for ByteArray<N> {
    type Error = FixedByteError;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        if value.len() != N {
            return Err(FixedByteError::InvalidLength);
        }
        Ok(ByteArray(value))
    }
}

impl<const N: usize> TryFrom<Vec<u8>> for ByteArray<N> {
    type Error = FixedByteError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() != N {
            return Err(FixedByteError::InvalidLength);
        }
        Ok(ByteArray(Bytes::from(value)))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[repr(transparent)]
pub struct ByteArrayVec<const N: usize>(Bytes);

#[cfg(feature = "serde")]
impl<'de, const N: usize> Deserialize<'de> for ByteArrayVec<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = Bytes::deserialize(deserializer)?;
        let len = bytes.len();
        if len % N == 0 {
            Ok(Self(bytes))
        } else {
            Err(serde::de::Error::invalid_length(
                len,
                &N.to_string().as_str(),
            ))
        }
    }
}

impl<const N: usize> ByteArrayVec<N> {
    pub fn len(&self) -> usize {
        self.0.len() / N
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn take_bytes(self) -> Bytes {
        self.0
    }

    /// Splits the byte array vec into two at the given index.
    ///
    /// Afterwards self contains elements [0, at), and the returned [`ByteArrayVec`] contains elements [at, len).
    ///
    /// This is an O(1) operation that just increases the reference count and sets a few indices.
    ///
    /// # Panics
    /// Panics if at > len.
    pub fn split_off(&mut self, at: usize) -> Self {
        Self(self.0.split_off(at * N))
    }
}

impl<const N: usize> From<&ByteArrayVec<N>> for Vec<[u8; N]> {
    fn from(value: &ByteArrayVec<N>) -> Self {
        let mut out = Vec::with_capacity(value.len());
        for i in 0..value.len() {
            out.push(value[i])
        }

        out
    }
}

impl<const N: usize> From<Vec<[u8; N]>> for ByteArrayVec<N> {
    fn from(value: Vec<[u8; N]>) -> Self {
        let mut bytes = BytesMut::with_capacity(N * value.len());
        for i in value.into_iter() {
            bytes.extend_from_slice(&i)
        }

        ByteArrayVec(bytes.freeze())
    }
}

impl<const N: usize> TryFrom<Bytes> for ByteArrayVec<N> {
    type Error = FixedByteError;

    fn try_from(value: Bytes) -> Result<Self, Self::Error> {
        if value.len() % N != 0 {
            return Err(FixedByteError::InvalidLength);
        }

        Ok(ByteArrayVec(value))
    }
}

impl<const N: usize> From<[u8; N]> for ByteArrayVec<N> {
    fn from(value: [u8; N]) -> Self {
        ByteArrayVec(Bytes::copy_from_slice(value.as_slice()))
    }
}

impl<const N: usize, const LEN: usize> From<[[u8; N]; LEN]> for ByteArrayVec<N> {
    fn from(value: [[u8; N]; LEN]) -> Self {
        let mut bytes = BytesMut::with_capacity(N * LEN);

        for val in value.into_iter() {
            bytes.put_slice(val.as_slice());
        }

        ByteArrayVec(bytes.freeze())
    }
}

impl<const N: usize> TryFrom<Vec<u8>> for ByteArrayVec<N> {
    type Error = FixedByteError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() % N != 0 {
            return Err(FixedByteError::InvalidLength);
        }

        Ok(ByteArrayVec(Bytes::from(value)))
    }
}

impl<const N: usize> Index<usize> for ByteArrayVec<N> {
    type Output = [u8; N];

    fn index(&self, index: usize) -> &Self::Output {
        if (index + 1) * N > self.0.len() {
            panic!("Index out of range, idx: {}, length: {}", index, self.len());
        }

        self.0[index * N..(index + 1) * N]
            .as_ref()
            .try_into()
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{from_str, to_string};

    use super::*;

    #[test]
    fn byte_array_vec_len() {
        let bytes = vec![0; 32 * 100];
        let bytes = ByteArrayVec::<32>::try_from(Bytes::from(bytes)).unwrap();

        assert_eq!(bytes.len(), 100);
        let _ = bytes[99];
    }

    /// Tests that `serde` works on [`ByteArray`].
    #[test]
    #[cfg(feature = "serde")]
    fn byte_array_serde() {
        let b = ByteArray::from([1, 0, 0, 0, 1]);
        let string = to_string(&b).unwrap();
        assert_eq!(string, "[1,0,0,0,1]");
        let b2 = from_str::<ByteArray<5>>(&string).unwrap();
        assert_eq!(b, b2);
    }

    /// Tests that `serde` works on [`ByteArrayVec`].
    #[test]
    #[cfg(feature = "serde")]
    fn byte_array_vec_serde() {
        let b = ByteArrayVec::from([1, 0, 0, 0, 1]);
        let string = to_string(&b).unwrap();
        assert_eq!(string, "[1,0,0,0,1]");
        let b2 = from_str::<ByteArrayVec<5>>(&string).unwrap();
        assert_eq!(b, b2);
    }

    /// Tests that bad input `serde` fails on [`ByteArray`].
    #[test]
    #[cfg(feature = "serde")]
    #[should_panic(
        expected = r#"called `Result::unwrap()` on an `Err` value: Error("invalid length 4, expected 5", line: 0, column: 0)"#
    )]
    fn byte_array_bad_deserialize() {
        from_str::<ByteArray<5>>("[1,0,0,0]").unwrap();
    }

    /// Tests that bad input `serde` fails on [`ByteArrayVec`].
    #[test]
    #[cfg(feature = "serde")]
    #[should_panic(
        expected = r#"called `Result::unwrap()` on an `Err` value: Error("invalid length 4, expected 5", line: 0, column: 0)"#
    )]
    fn byte_array_vec_bad_deserialize() {
        from_str::<ByteArrayVec<5>>("[1,0,0,0]").unwrap();
    }
}

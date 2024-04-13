use core::{
    fmt::{Debug, Formatter},
    ops::{Deref, Index},
};

use bytes::{BufMut, Bytes, BytesMut};

#[cfg_attr(feature = "std", derive(thiserror::Error))]
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
pub struct ByteArray<const N: usize>(Bytes);

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
pub struct ByteArrayVec<const N: usize>(Bytes);

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
    /// Afterwards self contains elements [0, at), and the returned Bytes contains elements [at, len).
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
    use super::*;

    #[test]
    fn byte_array_vec_len() {
        let bytes = vec![0; 32 * 100];
        let bytes = ByteArrayVec::<32>::try_from(Bytes::from(bytes)).unwrap();

        assert_eq!(bytes.len(), 100);
        let _ = bytes[99];
    }
}

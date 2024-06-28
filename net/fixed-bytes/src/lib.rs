use core::{
    fmt::{Debug, Formatter},
    ops::Deref,
};

use bytes::Bytes;

mod byte_array_vec;
pub use byte_array_vec::ByteArrayVec;

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

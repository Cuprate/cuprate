//! Hexadecimal serde wrappers for [`Vec<u8>`].
//!
//! This module provides transparent wrapper types for
//! arrays that (de)serialize from hexadecimal input/output.

use std::{
    borrow::Borrow,
    ops::{Deref, DerefMut},
};

use hex::FromHexError;
use serde::{Deserialize, Deserializer, Serialize};

/// Wrapper type for a byte [`Vec`] that (de)serializes from/to hexadecimal strings.
///
/// ```rust
/// # use cuprate_hex::HexVec;
/// let hash = [1; 32];
/// let hex_vec = HexVec::from(hash);
/// let expected_json = r#""0101010101010101010101010101010101010101010101010101010101010101""#;
///
/// let to_string = serde_json::to_string(&hex_vec).unwrap();
/// assert_eq!(to_string, expected_json);
///
/// let from_str = serde_json::from_str::<HexVec>(expected_json).unwrap();
/// assert_eq!(hex_vec, from_str);
///
/// //------
///
/// let vec = vec![hex_vec; 2];
/// let expected_json = r#"["0101010101010101010101010101010101010101010101010101010101010101","0101010101010101010101010101010101010101010101010101010101010101"]"#;
///
/// let to_string = serde_json::to_string(&vec).unwrap();
/// assert_eq!(to_string, expected_json);
///
/// let from_str = serde_json::from_str::<Vec<HexVec>>(expected_json).unwrap();
/// assert_eq!(vec, from_str);
/// ```
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct HexVec(#[serde(with = "hex::serde")] pub Vec<u8>);

impl HexVec {
    /// [`Vec::new`].
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Returns an empty [`Self`] if `array` is all `0`s.
    ///
    /// ```rust
    /// # use cuprate_hex::HexVec;
    /// assert_eq!(HexVec::empty_if_zeroed([1; 32]).0, [1; 32]);
    /// assert_eq!(HexVec::empty_if_zeroed([0; 32]), HexVec(vec![]));
    /// assert!(HexVec::empty_if_zeroed([0; 32]).is_empty());
    /// ```
    pub fn empty_if_zeroed<const N: usize>(array: [u8; N]) -> Self {
        if array == [0; N] {
            Self(Vec::new())
        } else {
            Self(array.to_vec())
        }
    }
}

impl<'de> Deserialize<'de> for HexVec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(hex::serde::deserialize(deserializer)?))
    }
}

impl Deref for HexVec {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for HexVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Borrow<Vec<u8>> for HexVec {
    fn borrow(&self) -> &Vec<u8> {
        &self.0
    }
}

impl AsRef<Vec<u8>> for HexVec {
    fn as_ref(&self) -> &Vec<u8> {
        &self.0
    }
}

impl From<HexVec> for Vec<u8> {
    fn from(hex: HexVec) -> Self {
        hex.0
    }
}

impl From<Vec<u8>> for HexVec {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl<const N: usize> From<[u8; N]> for HexVec {
    fn from(value: [u8; N]) -> Self {
        Self(value.to_vec())
    }
}

impl TryFrom<String> for HexVec {
    type Error = FromHexError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        hex::decode(value).map(Self)
    }
}

impl TryFrom<&str> for HexVec {
    type Error = FromHexError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        hex::decode(value).map(Self)
    }
}

impl<const N: usize> TryFrom<HexVec> for [u8; N] {
    type Error = FromHexError;
    fn try_from(value: HexVec) -> Result<Self, Self::Error> {
        Self::try_from(value.0).map_err(|_| FromHexError::InvalidStringLength)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn asdf() {
        let hash = vec![0; 32];
        let hex_vec = HexVec(hash);
        let expected_json = r#""0000000000000000000000000000000000000000000000000000000000000000""#;

        let to_string = serde_json::to_string(&hex_vec).unwrap();
        assert_eq!(to_string, expected_json);

        let from_str = serde_json::from_str::<HexVec>(expected_json).unwrap();
        assert_eq!(hex_vec, from_str);
    }
}

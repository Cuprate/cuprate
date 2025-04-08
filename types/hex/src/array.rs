//! Hexadecimal serde wrappers for arrays.
//!
//! This module provides transparent wrapper types for
//! arrays that (de)serialize from hexadecimal input/output.

use std::{
    borrow::Borrow,
    ops::{Deref, DerefMut},
};

use hex::{FromHex, FromHexError};
use serde::{Deserialize, Deserializer, Serialize};

/// Wrapper type for a byte array that (de)serializes from/to hexadecimal strings.
///
/// ```rust
/// # use cuprate_hex::Hex;
/// let hash = [1; 32];
/// let hex_bytes = Hex::<32>(hash);
/// let expected_json = r#""0101010101010101010101010101010101010101010101010101010101010101""#;
///
/// let to_string = serde_json::to_string(&hex_bytes).unwrap();
/// assert_eq!(to_string, expected_json);
///
/// let from_str = serde_json::from_str::<Hex<32>>(expected_json).unwrap();
/// assert_eq!(hex_bytes, from_str);
///
/// //------
///
/// let vec = vec![hex_bytes; 2];
/// let expected_json = r#"["0101010101010101010101010101010101010101010101010101010101010101","0101010101010101010101010101010101010101010101010101010101010101"]"#;
///
/// let to_string = serde_json::to_string(&vec).unwrap();
/// assert_eq!(to_string, expected_json);
///
/// let from_str = serde_json::from_str::<Vec<Hex<32>>>(expected_json).unwrap();
/// assert_eq!(vec, from_str);
/// ```
///
/// # Deserialization
/// This struct has a custom deserialization that only applies to certain
/// `N` lengths because [`FromHex`] does not implement for a generic `N`:
/// <https://docs.rs/hex/0.4.3/src/hex/lib.rs.html#220-230>
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct Hex<const N: usize>(#[serde(with = "hex::serde")] pub [u8; N]);

impl<const N: usize> Hex<N> {
    /// Returns `true` if the inner array is zeroed.
    ///
    /// ```rust
    /// # use cuprate_hex::Hex;
    /// assert!(Hex([0; 32]).is_zeroed());
    /// assert!(!Hex([1; 32]).is_zeroed());
    /// ```
    pub fn is_zeroed(&self) -> bool {
        *self == Self([0; N])
    }
}

impl<'de, const N: usize> Deserialize<'de> for Hex<N>
where
    [u8; N]: FromHex,
    <[u8; N] as FromHex>::Error: std::fmt::Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(hex::serde::deserialize(deserializer)?))
    }
}

// Default is not implemented for arrays >32, so we must do it manually.
impl<const N: usize> Default for Hex<N> {
    fn default() -> Self {
        Self([0; N])
    }
}

impl<const N: usize> Deref for Hex<N> {
    type Target = [u8; N];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> DerefMut for Hex<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<const N: usize> Borrow<[u8; N]> for Hex<N> {
    fn borrow(&self) -> &[u8; N] {
        &self.0
    }
}

impl<const N: usize> AsRef<[u8; N]> for Hex<N> {
    fn as_ref(&self) -> &[u8; N] {
        &self.0
    }
}

impl<const N: usize> From<Hex<N>> for [u8; N] {
    fn from(hex: Hex<N>) -> Self {
        hex.0
    }
}

impl<const N: usize> From<[u8; N]> for Hex<N> {
    fn from(value: [u8; N]) -> Self {
        Self(value)
    }
}

impl<const N: usize> TryFrom<String> for Hex<N> {
    type Error = FromHexError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let vec = hex::decode(value)?;
        match <[u8; N]>::try_from(vec) {
            Ok(s) => Ok(Self(s)),
            Err(_) => Err(FromHexError::InvalidStringLength),
        }
    }
}

impl<const N: usize> TryFrom<&str> for Hex<N> {
    type Error = FromHexError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut bytes = [0; N];
        hex::decode_to_slice(value, &mut bytes).map(|()| Self(bytes))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn asdf() {
        let hash = [0; 32];
        let hex_bytes = Hex::<32>(hash);
        let expected_json = r#""0000000000000000000000000000000000000000000000000000000000000000""#;

        let to_string = serde_json::to_string(&hex_bytes).unwrap();
        assert_eq!(to_string, expected_json);

        let from_str = serde_json::from_str::<Hex<32>>(expected_json).unwrap();
        assert_eq!(hex_bytes, from_str);
    }
}

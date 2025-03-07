//! Hexadecimal serde wrappers for arrays.
//!
//! This module provides transparent wrapper types for
//! arrays that (de)serialize from hexadecimal input/output.

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{EpeeValue, Marker, error, macros::bytes};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Wrapper type for a byte array that (de)serializes from/to hexadecimal strings.
///
/// # Deserialization
/// This struct has a custom deserialization that only applies to certain
/// `N` lengths because [`hex::FromHex`] does not implement for a generic `N`:
/// <https://docs.rs/hex/0.4.3/src/hex/lib.rs.html#220-230>
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[repr(transparent)]
pub struct HexBytes<const N: usize>(
    #[cfg_attr(feature = "serde", serde(with = "hex::serde"))] pub [u8; N],
);

#[cfg(feature = "serde")]
impl<'de, const N: usize> Deserialize<'de> for HexBytes<N>
where
    [u8; N]: hex::FromHex,
    <[u8; N] as hex::FromHex>::Error: std::fmt::Display,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self(hex::serde::deserialize(deserializer)?))
    }
}

#[cfg(feature = "epee")]
impl<const N: usize> EpeeValue for HexBytes<N> {
    const MARKER: Marker = <[u8; N] as EpeeValue>::MARKER;

    fn read<B: bytes::Buf>(r: &mut B, marker: &Marker) -> error::Result<Self> {
        Ok(Self(<[u8; N] as EpeeValue>::read(r, marker)?))
    }

    fn write<B: bytes::BufMut>(self, w: &mut B) -> error::Result<()> {
        <[u8; N] as EpeeValue>::write(self.0, w)
    }
}

// Default is not implemented for arrays >32, so we must do it manually.
impl<const N: usize> Default for HexBytes<N> {
    fn default() -> Self {
        Self([0; N])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn hex_bytes_32() {
        let hash = [1; 32];
        let hex_bytes = HexBytes::<32>(hash);
        let expected_json = r#""0101010101010101010101010101010101010101010101010101010101010101""#;

        let to_string = serde_json::to_string(&hex_bytes).unwrap();
        assert_eq!(to_string, expected_json);

        let from_str = serde_json::from_str::<HexBytes<32>>(expected_json).unwrap();
        assert_eq!(hex_bytes, from_str);
    }
}

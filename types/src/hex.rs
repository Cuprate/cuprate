//! Hexadecimal serde wrappers for arrays.
//!
//! This module provides transparent wrapper types for
//! arrays that (de)serialize from hexadecimal input/output.

#[cfg(feature = "epee")]
use cuprate_epee_encoding::{error, macros::bytes, EpeeValue, Marker};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Generate `HexBytes` struct(s) for `N` lengths.
///
/// This is a macro instead of a `<const N: usize>` implementation
/// because [`hex::FromHex`] does not implement for a generic `N`:
/// <https://docs.rs/hex/0.4.3/src/hex/lib.rs.html#220-230>
macro_rules! generate_hex_array {
    ($(
		$array_len:literal
	),* $(,)?) => { paste::paste! {
		$(
			#[doc = concat!(
				"Wrapper type for a ",
				stringify!($array_len),
				"-byte array that (de)serializes from/to hexadecimal strings."
			)]
			#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
			#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
			#[cfg_attr(feature = "serde", serde(transparent))]
			#[repr(transparent)]
			pub struct [<HexBytes $array_len>](
				#[cfg_attr(feature = "serde", serde(with = "hex::serde"))]
				pub [u8; $array_len],
			);

			#[cfg(feature = "epee")]
			impl EpeeValue for [<HexBytes $array_len>] {
				const MARKER: Marker = <[u8; $array_len] as EpeeValue>::MARKER;

				fn read<B: bytes::Buf>(r: &mut B, marker: &Marker) -> error::Result<Self> {
					Ok(Self(<[u8; $array_len] as EpeeValue>::read(r, marker)?))
				}

				fn write<B: bytes::BufMut>(self, w: &mut B) -> error::Result<()> {
					<[u8; $array_len] as EpeeValue>::write(self.0, w)
				}
			}

			// Default is not implemented for arrays >32, so must do it manually.
			impl Default for [<HexBytes $array_len>] {
				fn default() -> Self {
					Self([0; $array_len])
				}
			}
		)*
	}};
}

generate_hex_array!(1, 8, 32, 64);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn hex_bytes_32() {
        let hash = [1; 32];
        let hex_bytes = HexBytes32(hash);
        let expected_json = r#""0101010101010101010101010101010101010101010101010101010101010101""#;

        let to_string = serde_json::to_string(&hex_bytes).unwrap();
        assert_eq!(to_string, expected_json);

        let from_str = serde_json::from_str::<HexBytes32>(expected_json).unwrap();
        assert_eq!(hex_bytes, from_str);
    }
}

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Wrapper for fixed-size arrays of `u8` to provide serde serialization
/// and deserialization to and from hex strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Bytes<const N: usize>(
    #[serde(
        serialize_with = "serialize_to_hex",
        deserialize_with = "deserialize_from_hex"
    )]
    pub [u8; N],
);

fn serialize_to_hex<const N: usize, S>(bytes: &[u8; N], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&hex::encode(bytes))
}

fn deserialize_from_hex<'de, const N: usize, D>(deserializer: D) -> Result<[u8; N], D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let mut bytes = [0_u8; N];
    hex::decode_to_slice(s, &mut bytes).map_err(serde::de::Error::custom)?;
    Ok(bytes)
}

impl<const N: usize> fmt::Display for Bytes<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json;

    use super::*;

    #[test]
    fn test_bytes_json() {
        let json1 = "\"536f91da278f730f2524260d2778dc5959d40a5c724dd789d35bbd309eabd933\"";
        let array: Bytes<32> = serde_json::from_str(json1).unwrap();
        let json2 = serde_json::to_string(&array).unwrap();
        assert_eq!(json1, json2);
    }

    #[test]
    fn test_bytes_display() {
        let hex_str = "98f1e11d62b90c665a8a96fb1b10332e37a790ea1e01a9e8ec8de74b7b27b0df";
        let bytes = Bytes::<32>(hex::decode(hex_str).unwrap().try_into().unwrap());
        assert_eq!(format!("{}", bytes), hex_str);
    }
}

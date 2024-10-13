use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Bytes<const N: usize>(
    #[serde(
        serialize_with = "serialize_to_hex",
        deserialize_with = "deserialize_from_hex"
    )]
    [u8; N],
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
    fn test_bytes32_json() {
        let json1 = "\"536f91da278f730f2524260d2778dc5959d40a5c724dd789d35bbd309eabd933\"";
        let array: Bytes<32> = serde_json::from_str(json1).unwrap();
        let json2 = serde_json::to_string(&array).unwrap();
        assert_eq!(json1, json2);
    }
}

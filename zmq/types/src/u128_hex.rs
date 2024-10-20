//! Wrapper for u128 that serializes/deserializes to/from quoted hex
//! strings in big-endian order with a 0x prefix and no leading zeros.
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct U128(
    #[serde(
        serialize_with = "serialize_u128",
        deserialize_with = "deserialize_128"
    )]
    pub u128,
);

fn serialize_u128<S>(n: &u128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format!("0x{n:x}"))
}

fn deserialize_128<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: Deserializer<'de>,
{
    let mut s = String::deserialize(deserializer)?;
    if s.starts_with("0x") {
        s = s[2..].to_string();
    }
    u128::from_str_radix(&s, 16).map_err(serde::de::Error::custom)
}

impl fmt::Display for U128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:x}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u128_json() {
        let json1 = "\"0x123456789abcdef0123456789abcdef0\"";
        let n: U128 = serde_json::from_str(json1).unwrap();
        let json2 = serde_json::to_string(&n).unwrap();
        assert_eq!(json1, json2);
    }

    #[test]
    fn test_u128_display() {
        let n = U128(0x526f2623ce);
        assert_eq!(format!("{n}"), "0x526f2623ce");
    }
}

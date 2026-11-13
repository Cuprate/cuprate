//! Custom (de)serialization functions for serde.

//---------------------------------------------------------------------------------------------------- Lints
#![allow(clippy::trivially_copy_pass_by_ref)] // serde fn signature

//---------------------------------------------------------------------------------------------------- Import
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};

//---------------------------------------------------------------------------------------------------- Free functions
/// Always serializes `true`.
#[inline]
pub(crate) fn serde_true<S>(_: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_bool(true)
}

/// Always serializes `false`.
#[inline]
pub(crate) fn serde_false<S>(_: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_bool(false)
}

/// (De)serializes a value as a string of JSON, the form of monerod's nested JSON fields.
pub(crate) mod json_string {
    use super::{Deserialize, DeserializeOwned, Deserializer, Serialize, Serializer};

    pub(crate) fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        let json = serde_json::to_string_pretty(value).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&json)
    }

    pub(crate) fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: DeserializeOwned,
        D: Deserializer<'de>,
    {
        let json = String::deserialize(deserializer)?;
        serde_json::from_str(&json).map_err(serde::de::Error::custom)
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

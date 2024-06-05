//! TODO

//---------------------------------------------------------------------------------------------------- Use
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

//---------------------------------------------------------------------------------------------------- Version
/// JSON-RPC 2.0 Marker.
///
/// Always gets (de)serialized as [`Self::STR`].
///
/// TODO: <https://www.jsonrpc.org/specification#compatibility>.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version;

impl Version {
    /// TODO
    pub const STR: &'static str = "2.0";
}

//---------------------------------------------------------------------------------------------------- Trait impl
impl Serialize for Version {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(Self::STR)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", Self::STR)
    }
}

impl std::fmt::Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", Self::STR)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_str(VersionVisitor)
    }
}

//---------------------------------------------------------------------------------------------------- Serde impl
/// TODO
struct VersionVisitor;

impl Visitor<'_> for VersionVisitor {
    type Value = Version;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("Identifier must be the exact string: \"2.0\"")
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        match v {
            Version::STR => Ok(Version),
            _ => Err(Error::invalid_value(serde::de::Unexpected::Str(v), &self)),
        }
    }
}

//---------------------------------------------------------------------------------------------------- TEST
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    // Should always (de)serialize as "2.0".
    fn two_point_zero() {
        let s = serde_json::to_string(&Version).unwrap();
        assert_eq!(s, "\"2.0\"");

        let _: Version = serde_json::from_str(&s).unwrap();

        assert!(serde_json::from_str::<Version>("1.0").is_err());
        assert!(serde_json::from_str::<Version>("2.0").is_err()); // must be a string, not a float
        assert!(serde_json::from_str::<Version>("").is_err());
    }
}

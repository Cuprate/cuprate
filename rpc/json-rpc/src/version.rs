//! JSON-RPC 2.0 version marker.

//---------------------------------------------------------------------------------------------------- Use
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

//---------------------------------------------------------------------------------------------------- Version
/// [Protocol version marker](https://www.jsonrpc.org/specification#compatibility).
///
/// This represents the JSON-RPC version.
///
/// This is an empty marker type that always gets (de)serialized as [`Self::TWO`].
///
/// It is the only valid value for the `jsonrpc` field in the
/// [`Request`](crate::Request) and [`Response`](crate::Request) objects.
///
/// JSON-RPC 2.0 allows for backwards compatibility with `1.0` but this crate
/// (and this type) will not accept that, and will fail in deserialization
/// when encountering anything but [`Self::TWO`].
///
/// # Formatting
/// When using Rust formatting, [`Version`] is formatted as `2.0`.
///
/// When using JSON serialization, `Version` is formatted with quotes indicating
/// it is a JSON string and not a JSON float, i.e. it gets formatted as `"2.0"`, not `2.0`.
///
/// # Example
/// ```rust
/// use json_rpc::Version;
/// use serde_json::{to_string, to_string_pretty, from_str};
///
/// assert_eq!(Version::TWO, "2.0");
/// let version = Version;
///
/// // All debug/display formats are the same.
/// assert_eq!(format!("{version:?}"), Version::TWO);
/// assert_eq!(format!("{version:#?}"), Version::TWO);
/// assert_eq!(format!("{version}"), Version::TWO);
///
/// // JSON serialization will add extra quotes to
/// // indicate it is a string and not a float.
/// assert_eq!(to_string(&Version).unwrap(), "\"2.0\"");
/// assert_eq!(to_string_pretty(&Version).unwrap(), "\"2.0\"");
///
/// // Deserialization only accepts the JSON string "2.0".
/// assert!(from_str::<Version>(&"\"2.0\"").is_ok());
/// // This is JSON float, not a string.
/// assert!(from_str::<Version>(&"2.0").is_err());
///
/// assert!(from_str::<Version>(&"2").is_err());
/// assert!(from_str::<Version>(&"1.0").is_err());
/// assert!(from_str::<Version>(&"20").is_err());
/// assert!(from_str::<Version>(&"two").is_err());
/// assert!(from_str::<Version>(&"2.1").is_err());
/// assert!(from_str::<Version>(&"v2.0").is_err());
/// assert!(from_str::<Version>("").is_err());
/// ```
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version;

impl Version {
    /// The string `2.0`.
    ///
    /// Note that this does not have etra quotes to mark
    /// that it's a JSON string and not a float.
    /// ```rust
    /// use json_rpc::Version;
    ///
    /// let string = format!("{}", Version);
    /// assert_eq!(string, "2.0");
    /// assert_ne!(string, "\"2.0\"");
    /// ```
    pub const TWO: &'static str = "2.0";
}

//---------------------------------------------------------------------------------------------------- Trait impl
impl Serialize for Version {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(Self::TWO)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, r#"{}"#, Self::TWO)
    }
}

impl std::fmt::Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, r#"{}"#, Self::TWO)
    }
}

//---------------------------------------------------------------------------------------------------- Serde impl
/// Empty serde visitor for [`Version`].
struct VersionVisitor;

impl Visitor<'_> for VersionVisitor {
    type Value = Version;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("Identifier must be the exact string: \"2.0\"")
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        if v == Version::TWO {
            Ok(Version)
        } else {
            Err(Error::invalid_value(serde::de::Unexpected::Str(v), &self))
        }
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_str(VersionVisitor)
    }
}

//---------------------------------------------------------------------------------------------------- TEST
#[cfg(test)]
mod test {}

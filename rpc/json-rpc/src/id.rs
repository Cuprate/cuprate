//! [`Id`]: request/response identification.

//---------------------------------------------------------------------------------------------------- Use
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

//---------------------------------------------------------------------------------------------------- Id
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(untagged)]
/// [Request](crate::Request)/[Response](crate::Response) identification.
///
/// This is the [JSON-RPC 2.0 `id` field](https://www.jsonrpc.org/specification)
/// type found in `Request/Response`s.
///
/// # From
/// This type implements [`From`] on:
/// - [`String`]
/// - [`str`]
/// - [`u8`], [`u16`], [`u32`], [`u64`]
///
/// and all of those wrapped in [`Option`].
///
/// If the `Option` is [`None`], [`Id::Null`] is returned.
///
/// Note that the `&str` implementations will allocate, use [`Id::from_static_str`]
/// (or just manually create the `Cow`) for a non-allocating `Id`.
///
/// ```rust
/// use json_rpc::Id;
///
/// assert_eq!(Id::from(String::new()), Id::Str("".into()));
/// assert_eq!(Id::from(Some(String::new())), Id::Str("".into()));
/// assert_eq!(Id::from(None::<String>), Id::Null);
/// assert_eq!(Id::from(123_u64), Id::Num(123_u64));
/// assert_eq!(Id::from(Some(123_u64)), Id::Num(123_u64));
/// assert_eq!(Id::from(None::<u64>), Id::Null);
/// ```
pub enum Id {
    /// A JSON `null` value.
    ///
    /// ```rust
    /// use json_rpc::Id;
    /// use serde_json::{from_value,to_value,json,Value};
    ///
    /// assert_eq!(from_value::<Id>(json!(null)).unwrap(), Id::Null);
    /// assert_eq!(to_value(Id::Null).unwrap(), Value::Null);
    ///
    /// // Not a real `null`, but a string.
    /// assert_eq!(from_value::<Id>(json!("null")).unwrap(), Id::Str("null".into()));
    /// ```
    Null,

    /// A JSON `number` value.
    Num(u64),

    /// A JSON `string` value.
    ///
    /// This is a `Cow<'static, str>` to support both 0-allocation for
    /// `const` string ID's commonly found in programs, as well as support
    /// for runtime [`String`]'s.
    ///
    /// ```rust
    /// use std::borrow::Cow;
    /// use json_rpc::Id;
    ///
    /// /// A program's static ID.
    /// const ID: &'static str = "my_id";
    ///
    /// // No allocation.
    /// let s = Id::Str(Cow::Borrowed(ID));
    ///
    /// // Runtime allocation.
    /// let s = Id::Str(Cow::Owned("runtime_id".to_string()));
    /// ```
    Str(Cow<'static, str>),
}

impl Id {
    /// This returns `Some(u64)` if [`Id`] is a number.
    ///
    /// ```rust
    /// use json_rpc::Id;
    ///
    /// assert_eq!(Id::Num(0).as_u64(), Some(0));
    /// assert_eq!(Id::Str("0".into()).as_u64(), None);
    /// assert_eq!(Id::Null.as_u64(), None);
    /// ```
    pub const fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Num(n) => Some(*n),
            _ => None,
        }
    }

    /// This returns `Some(&str)` if [`Id`] is a string.
    ///
    /// ```rust
    /// use json_rpc::Id;
    ///
    /// assert_eq!(Id::Str("0".into()).as_str(), Some("0"));
    /// assert_eq!(Id::Num(0).as_str(), None);
    /// assert_eq!(Id::Null.as_str(), None);
    /// ```
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s.as_ref()),
            _ => None,
        }
    }

    /// Returns `true` if `self` is [`Id::Null`].
    ///
    /// ```rust
    /// use json_rpc::Id;
    ///
    /// assert!(Id::Null.is_null());
    /// assert!(!Id::Num(0).is_null());
    /// assert!(!Id::Str("".into()).is_null());
    /// ```
    pub fn is_null(&self) -> bool {
        *self == Self::Null
    }

    /// Create a new [`Id::Str`] from a static string.
    ///
    /// ```rust
    /// use json_rpc::Id;
    ///
    /// assert_eq!(Id::from_static_str("hi"), Id::Str("hi".into()));
    /// ```
    pub const fn from_static_str(s: &'static str) -> Self {
        Self::Str(Cow::Borrowed(s))
    }

    /// Inner infallible implementation of [`FromStr::from_str`]
    const fn from_string(s: String) -> Self {
        Self::Str(Cow::Owned(s))
    }
}

impl std::str::FromStr for Id {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, std::convert::Infallible> {
        Ok(Self::from_string(s.to_string()))
    }
}

impl From<String> for Id {
    fn from(s: String) -> Self {
        Self::from_string(s)
    }
}

impl From<&str> for Id {
    fn from(s: &str) -> Self {
        Self::from_string(s.to_string())
    }
}

impl From<Option<String>> for Id {
    fn from(s: Option<String>) -> Self {
        match s {
            Some(s) => Self::from_string(s),
            None => Self::Null,
        }
    }
}

impl From<Option<&str>> for Id {
    fn from(s: Option<&str>) -> Self {
        let s = s.map(ToString::to_string);
        s.into()
    }
}

/// Implement `From<unsigned integer>` for `Id`.
///
/// Not a generic since that clashes with `From<String>`.
macro_rules! impl_u {
    ($($u:ty),*) => {
        $(
            impl From<$u> for Id {
                fn from(u: $u) -> Self {
                    Self::Num(u as u64)
                }
            }

            impl From<&$u> for Id {
                fn from(u: &$u) -> Self {
                    Self::Num(*u as u64)
                }
            }

            impl From<Option<$u>> for Id {
                fn from(u: Option<$u>) -> Self {
                    match u {
                        Some(u) => Self::Num(u as u64),
                        None => Self::Null,
                    }
                }
            }
        )*
    }
}

impl_u!(u8, u16, u32);
#[cfg(target_pointer_width = "64")]
impl_u!(u64);

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod test {
    use super::*;

    /// Basic [`Id::as_u64()`] tests.
    #[test]
    fn __as_u64() {
        let id = Id::Num(u64::MIN);
        assert_eq!(id.as_u64().unwrap(), u64::MIN);

        let id = Id::Num(u64::MAX);
        assert_eq!(id.as_u64().unwrap(), u64::MAX);

        let id = Id::Null;
        assert!(id.as_u64().is_none());
        let id = Id::Str("".into());
        assert!(id.as_u64().is_none());
    }

    /// Basic [`Id::as_str()`] tests.
    #[test]
    fn __as_str() {
        let id = Id::Str("str".into());
        assert_eq!(id.as_str().unwrap(), "str");

        let id = Id::Null;
        assert!(id.as_str().is_none());
        let id = Id::Num(0);
        assert!(id.as_str().is_none());
    }
}

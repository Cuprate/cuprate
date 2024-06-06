//! TODO

//---------------------------------------------------------------------------------------------------- Use
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

//---------------------------------------------------------------------------------------------------- Id
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(untagged)]
/// [Request/Response object ID](https://www.jsonrpc.org/specification)
pub enum Id {
    /// `null`
    Null,

    /// Number ID
    Num(u64),

    /// String ID
    Str(Cow<'static, str>),
}

impl Id {
    #[inline]
    /// Return inner [`u64`] if [`Id`] is a number
    pub const fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Num(n) => Some(*n),
            _ => None,
        }
    }

    #[inline]
    /// Return inner [`str`] if [`Id`] is a string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(s) => Some(s.as_ref()),
            _ => None,
        }
    }

    #[inline]
    /// TODO
    pub fn is_null(&self) -> bool {
        *self == Self::Null
    }

    #[inline]
    /// Extract the underlying number from the [`Id`].
    pub fn try_parse_num(&self) -> Option<u64> {
        match self {
            Self::Null => None,
            Self::Num(num) => Some(*num),
            Self::Str(s) => s.parse().ok(),
        }
    }

    /// TODO
    fn from_string(s: String) -> Self {
        if let Ok(u) = s.parse::<u64>() {
            return Self::Num(u);
        }

        match s.as_str() {
            "null" | "Null" | "NULL" => Self::Null,
            _ => Self::Str(Cow::Owned(s)),
        }
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

/// TODO
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
		)*
	}
}

impl_u!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn null() {
        let id = Id::Null;
        assert!(id.is_null());
    }

    #[test]
    fn parse() {
        let id = Id::Str(format!("{}", u64::MIN).into());
        assert_eq!(id.try_parse_num().unwrap(), u64::MIN);

        let id = Id::Str(format!("{}", u64::MAX).into());
        assert_eq!(id.try_parse_num().unwrap(), u64::MAX);

        let id = Id::Str(format!("{}a", u64::MAX).into());
        assert!(id.try_parse_num().is_none());

        let id = Id::Num(u64::MIN);
        assert_eq!(id.try_parse_num().unwrap(), u64::MIN);

        let id = Id::Num(u64::MAX);
        assert_eq!(id.try_parse_num().unwrap(), u64::MAX);

        let id = Id::Null;
        assert!(id.try_parse_num().is_none());
    }

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

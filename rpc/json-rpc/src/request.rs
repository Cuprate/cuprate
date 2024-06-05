//! TODO

//---------------------------------------------------------------------------------------------------- Use
use crate::id::Id;
use crate::version::Version;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

//---------------------------------------------------------------------------------------------------- Request
/// JSON-RPC 2.0 Request object
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request<'a, B> {
    /// JSON-RPC 2.0
    pub jsonrpc: Version,

    #[serde(borrow)]
    /// An identifier established by the Client that MUST contain a String, Number, or NULL value if included.
    ///
    /// If it is not included it is assumed to be a notification.
    pub id: Id<'a>,

    #[serde(flatten)]
    /// TODO
    ///
    /// method: A type that serializes as the name of the method to be invoked.
    ///
    /// params: A Structured value that holds the parameter values to be used during the invocation of the method.
    pub body: B,
}

impl<'a, B> Request<'a, B> {
    #[inline]
    /// Create a new [`Self`].
    pub const fn new(id: Id<'a>, body: B) -> Self {
        Self {
            jsonrpc: Version,
            id,
            body,
        }
    }

    #[inline]
    /// Returns whether request is notification.
    pub fn is_notification(&self) -> bool {
        self.id.is_null()
    }
}

//---------------------------------------------------------------------------------------------------- Trait impl
impl<B> std::fmt::Display for Request<'_, B>
where
    B: std::fmt::Display + Clone + Serialize,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string_pretty(self) {
            Ok(json) => write!(f, "{json}"),
            Err(_) => Err(std::fmt::Error),
        }
    }
}

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod test {
    use super::*;
    use crate::id::Id;

    use pretty_assertions::assert_eq;

    #[test]
    fn serde() {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct Body {
            method: String,
            params: [u8; 3],
        }

        let id = Id::Num(123);
        let body = Body {
            method: String::from("a_method"),
            params: [0, 1, 2],
        };

        let req = Request::new(id.clone(), body);

        assert!(!req.is_notification());

        let ser: String = serde_json::to_string(&req).unwrap();
        let de: Request<Body> = serde_json::from_str(&ser).unwrap();

        assert_eq!(req, de);
    }
}

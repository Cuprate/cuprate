//! JSON-RPC 2.0 request object.

//---------------------------------------------------------------------------------------------------- Use
use serde::{Deserialize, Serialize};

use crate::{id::Id, version::Version};

//---------------------------------------------------------------------------------------------------- Request
/// [The request object](https://www.jsonrpc.org/specification#request_object).
///
/// The generic `T` is the body type of the request, i.e. it is the
/// type that holds both the `method` and `params`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request<T> {
    /// JSON-RPC protocol version; always `2.0`.
    pub jsonrpc: Version,

    /// An identifier established by the Client.
    ///
    /// If it is not included it is assumed to be a notification.
    ///
    /// # `None` vs `Some(Id::Null)`
    /// This field will be completely omitted during serialization if [`None`],
    /// however if it is `Some(Id::Null)`, it will be serialized as `"id": null`.
    ///
    /// Note that the JSON-RPC 2.0 specification discourages the use of `Id::NUll`,
    /// so if there is no ID needed, consider using `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,

    #[serde(flatten)]
    /// The `method` and `params` fields.
    ///
    /// - `method`: A type that serializes as the name of the method to be invoked.
    /// - `params`: A structured value that holds the parameter values to be used during the invocation of the method.
    ///
    /// As mentioned in the library documentation, there are no `method/params` fields in [`Request`],
    /// they are both merged in this `body` field which is `#[serde(flatten)]`ed.
    ///
    /// # Invariant
    /// Your `T` must serialize as `method` and `params` to comply with the specification.
    pub body: T,
}

impl<T> Request<T> {
    #[inline]
    /// Create a new [`Self`] with no [`Id`].
    ///
    /// ```rust
    /// use json_rpc::Request;
    ///
    /// assert_eq!(Request::new("").id, None);
    /// ```
    pub const fn new(body: T) -> Self {
        Self {
            jsonrpc: Version,
            id: None,
            body,
        }
    }

    #[inline]
    /// Create a new [`Self`] with an [`Id`].
    ///
    /// ```rust
    /// use json_rpc::{Id, Request};
    ///
    /// assert_eq!(Request::new_with_id(Id::Num(0), "").id, Some(Id::Num(0)));
    /// ```
    pub const fn new_with_id(id: Id, body: T) -> Self {
        Self {
            jsonrpc: Version,
            id: Some(id),
            body,
        }
    }

    #[inline]
    /// Returns `true` if the request is [notification](https://www.jsonrpc.org/specification#notification).
    ///
    /// In other words, if `id` is [`None`], this returns `true`.
    ///
    /// ```rust
    /// use json_rpc::{Id, Request};
    ///
    /// assert!(Request::new("").is_notification());
    /// assert!(!Request::new_with_id(Id::Null, "").is_notification());
    /// ```
    pub const fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

//---------------------------------------------------------------------------------------------------- Trait impl
impl<T> std::fmt::Display for Request<T>
where
    T: std::fmt::Display + Clone + Serialize,
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
    use crate::{
        id::Id,
        tests::{assert_serde, Body},
    };

    use pretty_assertions::assert_eq;
    use serde_json::{json, Value};

    /// Basic serde tests.
    #[test]
    fn serde() {
        let id = Id::Num(123);
        let body = Body {
            method: "a_method".into(),
            params: [0, 1, 2],
        };

        let req = Request::new_with_id(id, body);

        assert!(!req.is_notification());

        let ser: String = serde_json::to_string(&req).unwrap();
        let de: Request<Body<[u8; 3]>> = serde_json::from_str(&ser).unwrap();

        assert_eq!(req, de);
    }

    /// Tests that null `id` shows when serializing.
    #[test]
    fn request_null_id() {
        let req = Request::new_with_id(
            Id::Null,
            Body {
                method: "m".into(),
                params: "p".to_string(),
            },
        );
        let json = json!({
            "jsonrpc": "2.0",
            "id": null,
            "method": "m",
            "params": "p",
        });

        assert_serde(&req, &json);
    }

    /// Tests that a `None` `id` omits the field when serializing.
    #[test]
    fn request_none_id() {
        let req = Request::new(Body {
            method: "a".into(),
            params: "b".to_string(),
        });
        let json = json!({
            "jsonrpc": "2.0",
            "method": "a",
            "params": "b",
        });

        assert_serde(&req, &json);
    }

    /// Tests that omitting `params` omits the field when serializing.
    #[test]
    fn request_no_params() {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct NoParamMethod {
            method: String,
        }

        let req = Request::new_with_id(
            Id::Num(123),
            NoParamMethod {
                method: "asdf".to_string(),
            },
        );
        let json = json!({
            "jsonrpc": "2.0",
            "id": 123,
            "method": "asdf",
        });

        assert_serde(&req, &json);
    }

    /// Tests that tagged enums serialize correctly.
    #[test]
    fn request_tagged_enums() {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct GetHeight {
            height: u64,
        }

        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        #[serde(tag = "method", content = "params")]
        #[serde(rename_all = "snake_case")]
        enum Methods {
            GetHeight(/* param: */ GetHeight),
        }

        let req = Request::new_with_id(Id::Num(123), Methods::GetHeight(GetHeight { height: 0 }));
        let json = json!({
            "jsonrpc": "2.0",
            "id": 123,
            "method": "get_height",
            "params": {
                "height": 0,
            },
        });

        assert_serde(&req, &json);
    }

    /// Tests that requests serialize into the expected JSON value.
    #[test]
    fn request_is_expected_value() {
        // Test values: (request, expected_value)
        let array: [(Request<Body<[u8; 3]>>, Value); 3] = [
            (
                Request::new_with_id(
                    Id::Num(123),
                    Body {
                        method: "method_1".into(),
                        params: [0, 1, 2],
                    },
                ),
                json!({
                    "jsonrpc": "2.0",
                    "id": 123,
                    "method": "method_1",
                    "params": [0, 1, 2],
                }),
            ),
            (
                Request::new_with_id(
                    Id::Null,
                    Body {
                        method: "method_2".into(),
                        params: [3, 4, 5],
                    },
                ),
                json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "method": "method_2",
                    "params": [3, 4, 5],
                }),
            ),
            (
                Request::new_with_id(
                    Id::Str("string_id".into()),
                    Body {
                        method: "method_3".into(),
                        params: [6, 7, 8],
                    },
                ),
                json!({
                    "jsonrpc": "2.0",
                    "method": "method_3",
                    "id": "string_id",
                    "params": [6, 7, 8],
                }),
            ),
        ];

        for (request, expected_value) in array {
            assert_serde(&request, &expected_value);
        }
    }
}

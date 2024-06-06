//! TODO

//---------------------------------------------------------------------------------------------------- Use
use serde::{Deserialize, Serialize};

use crate::{id::Id, version::Version};

//---------------------------------------------------------------------------------------------------- Request
/// JSON-RPC 2.0 Request object
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Request<T> {
    /// JSON-RPC 2.0
    pub jsonrpc: Version,

    /// An identifier established by the Client that MUST contain a String, Number, or NULL value if included.
    ///
    /// If it is not included it is assumed to be a notification.
    pub id: Id,

    #[serde(flatten)]
    /// TODO
    ///
    /// method: A type that serializes as the name of the method to be invoked.
    ///
    /// params: A Structured value that holds the parameter values to be used during the invocation of the method.
    pub body: T,
}

impl<T> Request<T> {
    #[inline]
    /// Create a new [`Self`].
    pub const fn new(id: Id, body: T) -> Self {
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

        let req = Request::new(id, body);

        assert!(!req.is_notification());

        let ser: String = serde_json::to_string(&req).unwrap();
        let de: Request<Body<[u8; 3]>> = serde_json::from_str(&ser).unwrap();

        assert_eq!(req, de);
    }

    /// Tests that omitting `params` omits the field when serializing.
    #[test]
    fn request_no_params() {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        struct NoParamMethod {
            method: String,
        }

        let req = Request::new(
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

        let req = Request::new(Id::Num(123), Methods::GetHeight(GetHeight { height: 0 }));
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
                Request::new(
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
                Request::new(
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
                Request::new(
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

//! JSON-RPC 2.0 response object.

//---------------------------------------------------------------------------------------------------- Use
use serde::{ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};

use crate::{error::ErrorObject, id::Id, version::Version};

//---------------------------------------------------------------------------------------------------- Response
/// [The response object](https://www.jsonrpc.org/specification#response_object).
///
/// The generic `T` is the response payload, i.e. it is the
/// type that holds both the `method` and `params`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response<T> {
    /// JSON-RPC protocol version; always `2.0`.
    pub jsonrpc: Version,

    /// This field must always be present in serialized JSON.
    ///
    /// ### JSON-RPC 2.0 rules
    /// - The [`Response`]'s ID must be the same as the [`Request`](crate::Request)
    /// - If the `Request` omitted the `id` field, there should be no `Response`
    /// - If there was an error in detecting the `Request`'s ID, the `Response` must contain an [`Id::Null`]
    pub id: Id,

    /// The response payload.
    ///
    /// ### JSON-RPC 2.0 rules
    /// - This must be [`Ok`] upon success
    /// - This must be [`Err`] upon error
    /// - This can be any (de)serializable data `T` on success
    /// - This must be [`ErrorObject`] on errors
    pub payload: Result<T, ErrorObject>,
}

impl<T> Response<T> {
    /// Creates a successful response.
    ///
    /// ```rust
    /// use json_rpc::{Id, Response};
    ///
    /// let ok = Response::ok(Id::Num(123), "OK");
    /// let json = serde_json::to_string(&ok).unwrap();
    /// assert_eq!(json, r#"{"jsonrpc":"2.0","id":123,"result":"OK"}"#);
    /// ```
    pub const fn ok(id: Id, result: T) -> Self {
        Self {
            jsonrpc: Version,
            id,
            payload: Ok(result),
        }
    }

    /// Creates an error response.
    ///
    /// ```rust
    /// use json_rpc::{Id, Response, error::{ErrorObject, ErrorCode}};
    ///
    /// let err = ErrorObject {
    ///     code: 0.into(),
    ///     message: "m".into(),
    ///     data: Some("d".into()),
    /// };
    ///
    /// let ok = Response::<()>::err(Id::Num(123), err);
    /// let json = serde_json::to_string(&ok).unwrap();
    /// assert_eq!(json, r#"{"jsonrpc":"2.0","id":123,"error":{"code":0,"message":"m","data":"d"}}"#);
    /// ```
    pub const fn err(id: Id, error: ErrorObject) -> Self {
        Self {
            jsonrpc: Version,
            id,
            payload: Err(error),
        }
    }

    /// Creates an error response using [`ErrorObject::parse_error`].
    ///
    /// ```rust
    /// use json_rpc::{Id, Response, error::{ErrorObject, ErrorCode}};
    ///
    /// let ok = Response::<()>::parse_error(Id::Num(0));
    /// let json = serde_json::to_string(&ok).unwrap();
    /// assert_eq!(json, r#"{"jsonrpc":"2.0","id":0,"error":{"code":-32700,"message":"Parse error"}}"#);
    /// ```
    pub const fn parse_error(id: Id) -> Self {
        Self {
            jsonrpc: Version,
            payload: Err(ErrorObject::parse_error()),
            id,
        }
    }

    /// Creates an error response using [`ErrorObject::invalid_request`].
    ///
    /// ```rust
    /// use json_rpc::{Id, Response, error::{ErrorObject, ErrorCode}};
    ///
    /// let ok = Response::<()>::invalid_request(Id::Num(0));
    /// let json = serde_json::to_string(&ok).unwrap();
    /// assert_eq!(json, r#"{"jsonrpc":"2.0","id":0,"error":{"code":-32600,"message":"Invalid Request"}}"#);
    /// ```
    pub const fn invalid_request(id: Id) -> Self {
        Self {
            jsonrpc: Version,
            payload: Err(ErrorObject::invalid_request()),
            id,
        }
    }

    /// Creates an error response using [`ErrorObject::method_not_found`].
    ///
    /// ```rust
    /// use json_rpc::{Id, Response, error::{ErrorObject, ErrorCode}};
    ///
    /// let ok = Response::<()>::method_not_found(Id::Num(0));
    /// let json = serde_json::to_string(&ok).unwrap();
    /// assert_eq!(json, r#"{"jsonrpc":"2.0","id":0,"error":{"code":-32601,"message":"Method not found"}}"#);
    /// ```
    pub const fn method_not_found(id: Id) -> Self {
        Self {
            jsonrpc: Version,
            payload: Err(ErrorObject::method_not_found()),
            id,
        }
    }

    /// Creates an error response using [`ErrorObject::invalid_params`].
    ///
    /// ```rust
    /// use json_rpc::{Id, Response, error::{ErrorObject, ErrorCode}};
    ///
    /// let ok = Response::<()>::invalid_params(Id::Num(0));
    /// let json = serde_json::to_string(&ok).unwrap();
    /// assert_eq!(json, r#"{"jsonrpc":"2.0","id":0,"error":{"code":-32602,"message":"Invalid params"}}"#);
    /// ```
    pub const fn invalid_params(id: Id) -> Self {
        Self {
            jsonrpc: Version,
            payload: Err(ErrorObject::invalid_params()),
            id,
        }
    }

    /// Creates an error response using [`ErrorObject::internal_error`].
    ///
    /// ```rust
    /// use json_rpc::{Id, Response, error::{ErrorObject, ErrorCode}};
    ///
    /// let ok = Response::<()>::internal_error(Id::Num(0));
    /// let json = serde_json::to_string(&ok).unwrap();
    /// assert_eq!(json, r#"{"jsonrpc":"2.0","id":0,"error":{"code":-32603,"message":"Internal error"}}"#);
    /// ```
    pub const fn internal_error(id: Id) -> Self {
        Self {
            jsonrpc: Version,
            payload: Err(ErrorObject::internal_error()),
            id,
        }
    }
}

//---------------------------------------------------------------------------------------------------- Trait impl
impl<T> std::fmt::Display for Response<T>
where
    T: Serialize,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string_pretty(self) {
            Ok(json) => write!(f, "{json}"),
            Err(_) => Err(std::fmt::Error),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Serde impl
impl<T> Serialize for Response<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("Response", 3)?;

        s.serialize_field("jsonrpc", &self.jsonrpc)?;

        // This member is required.
        //
        // Even if `null`, or the client `Request` didn't include one.
        s.serialize_field("id", &self.id)?;

        match &self.payload {
            Ok(r) => s.serialize_field("result", r)?,
            Err(e) => s.serialize_field("error", e)?,
        }

        s.end()
    }
}

// [`Response`] has a manual deserialization implementation because
// we need to confirm `result` and `error` don't both exist:
//
// > Either the result member or error member MUST be included, but both members MUST NOT be included.
//
// <https://www.jsonrpc.org/specification#error_object>
impl<'de, T> Deserialize<'de> for Response<T>
where
    T: Deserialize<'de> + 'de,
{
    fn deserialize<D: Deserializer<'de>>(der: D) -> Result<Self, D::Error> {
        use std::marker::PhantomData;

        use serde::de::{Error, MapAccess, Visitor};

        /// This type represents the key values within [`Response`].
        enum Key {
            /// "jsonrpc" field.
            JsonRpc,
            /// "result" field.
            Result,
            /// "error" field.
            Error,
            /// "id" field.
            Id,
            /// Any other unknown field (ignored).
            Unknown,
        }

        // Deserialization for [`Response`]'s key fields.
        //
        // This ignores unknown keys.
        impl<'de> Deserialize<'de> for Key {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                /// Serde visitor for [`Response`]'s key fields.
                struct KeyVisitor;

                impl Visitor<'_> for KeyVisitor {
                    type Value = Key;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("`jsonrpc`, `id`, `result`, `error`")
                    }

                    fn visit_str<E>(self, string: &str) -> Result<Key, E>
                    where
                        E: Error,
                    {
                        // PERF: this match is in order of how this library serializes fields.
                        match string {
                            "jsonrpc" => Ok(Key::JsonRpc),
                            "id" => Ok(Key::Id),
                            "result" => Ok(Key::Result),
                            "error" => Ok(Key::Error),
                            // Ignore any other keys that appear
                            // and continue deserialization.
                            _ => Ok(Key::Unknown),
                        }
                    }
                }

                deserializer.deserialize_identifier(KeyVisitor)
            }
        }

        /// Serde visitor for the key-value map of [`Response`].
        struct MapVisit<T>(PhantomData<T>);

        // Deserialization for [`Response`]'s key and values (the JSON map).
        impl<'de, T> Visitor<'de> for MapVisit<T>
        where
            T: Deserialize<'de> + 'de,
        {
            type Value = Response<T>;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str("JSON-RPC 2.0 Response")
            }

            /// This is a loop that goes over every key-value pair
            /// and fills out the necessary fields.
            ///
            /// If both `result/error` appear then this
            /// deserialization will error, as to
            /// follow the JSON-RPC 2.0 specification.
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                // Initialize values.
                let mut jsonrpc = None;
                let mut payload = None;
                let mut id = None;

                // Loop over map, filling values.
                while let Some(key) = map.next_key::<Key>()? {
                    // PERF: this match is in order of how this library serializes fields.
                    match key {
                        Key::JsonRpc => jsonrpc = Some(map.next_value::<Version>()?),
                        Key::Id => id = Some(map.next_value::<Id>()?),
                        Key::Result => {
                            if payload.is_none() {
                                payload = Some(Ok(map.next_value::<T>()?));
                            } else {
                                return Err(serde::de::Error::duplicate_field("result/error"));
                            }
                        }
                        Key::Error => {
                            if payload.is_none() {
                                payload = Some(Err(map.next_value::<ErrorObject>()?));
                            } else {
                                return Err(serde::de::Error::duplicate_field("result/error"));
                            }
                        }
                        Key::Unknown => {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                // Make sure all our key-value pairs are set and correct.
                match (jsonrpc, id, payload) {
                    // Response with a single `result` or `error`.
                    (Some(jsonrpc), Some(id), Some(payload)) => Ok(Response {
                        jsonrpc,
                        id,
                        payload,
                    }),

                    // No fields existed.
                    (None, None, None) => Err(Error::missing_field("jsonrpc + id + result/error")),

                    // Some field was missing.
                    (None, _, _) => Err(Error::missing_field("jsonrpc")),
                    (_, None, _) => Err(Error::missing_field("id")),
                    (_, _, None) => Err(Error::missing_field("result/error")),
                }
            }
        }

        /// All expected fields of the [`Response`] type.
        const FIELDS: &[&str; 4] = &["jsonrpc", "id", "result", "error"];
        der.deserialize_struct("Response", FIELDS, MapVisit(PhantomData))
    }
}

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;
    use crate::id::Id;

    /// Basic serde test on OK results.
    #[test]
    fn serde_result() {
        let result = String::from("result_ok");
        let id = Id::Num(123);
        let req = Response::ok(id.clone(), result.clone());

        let ser: String = serde_json::to_string(&req).unwrap();
        let de: Response<String> = serde_json::from_str(&ser).unwrap();

        assert_eq!(de.payload.unwrap(), result);
        assert_eq!(de.id, id);
    }

    /// Basic serde test on errors.
    #[test]
    fn serde_error() {
        let error = ErrorObject::internal_error();
        let id = Id::Num(123);
        let req: Response<String> = Response::err(id.clone(), error.clone());

        let ser: String = serde_json::to_string(&req).unwrap();
        let de: Response<String> = serde_json::from_str(&ser).unwrap();

        assert_eq!(de.payload.unwrap_err(), error);
        assert_eq!(de.id, id);
    }

    /// Test that the `result` and `error` fields are mutually exclusive.
    #[test]
    #[should_panic(
        expected = "called `Result::unwrap()` on an `Err` value: Error(\"duplicate field `result/error`\", line: 0, column: 0)"
    )]
    fn result_error_mutually_exclusive() {
        let e = ErrorObject::internal_error();
        let j = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "result": "",
            "error": e
        });
        serde_json::from_value::<Response<String>>(j).unwrap();
    }

    /// Test that the `result` and `error` fields can repeat (and get overwritten).
    #[test]
    #[should_panic(
        expected = "called `Result::unwrap()` on an `Err` value: Error(\"duplicate field `result/error`\", line: 1, column: 45)"
    )]
    fn result_repeat() {
        // `result`
        let json = r#"{"jsonrpc":"2.0","id":0,"result":"a","result":"b"}"#;
        serde_json::from_str::<Response<String>>(json).unwrap();
    }

    /// Test that the `error` field cannot repeat.
    #[test]
    #[should_panic(
        expected = "called `Result::unwrap()` on an `Err` value: Error(\"duplicate field `result/error`\", line: 1, column: 83)"
    )]
    fn error_repeat() {
        let e = ErrorObject::invalid_request();
        let e = serde_json::to_string(&e).unwrap();
        let json = format!(r#"{{"jsonrpc":"2.0","id":0,"error":{e},"error":{e}}}"#);
        serde_json::from_str::<Response<String>>(&json).unwrap();
    }

    /// Test that the `id` field must exist.
    #[test]
    #[should_panic(
        expected = "called `Result::unwrap()` on an `Err` value: Error(\"missing field `id`\", line: 0, column: 0)"
    )]
    fn id_must_exist() {
        let j = json!({
            "jsonrpc": "2.0",
            "result": "",
        });
        serde_json::from_value::<Response<String>>(j).unwrap();
    }

    /// Tests that non-ordered fields still deserialize okay.
    #[test]
    fn deserialize_out_of_order_keys() {
        let e = ErrorObject::internal_error();
        let j = json!({
            "error": e,
            "id": 0,
            "jsonrpc": "2.0"
        });
        let resp = serde_json::from_value::<Response<String>>(j).unwrap();
        assert_eq!(resp, Response::internal_error(Id::Num(0)));

        let ok = Response::ok(Id::Num(0), "OK".to_string());
        let j = json!({
            "result": "OK",
            "id": 0,
            "jsonrpc": "2.0"
        });
        let resp = serde_json::from_value::<Response<String>>(j).unwrap();
        assert_eq!(resp, ok);
    }

    /// Asserts that fields must be `lowercase`.
    #[test]
    #[should_panic(
        expected = "called `Result::unwrap()` on an `Err` value: Error(\"missing field `jsonrpc`\", line: 1, column: 40)"
    )]
    fn lowercase() {
        let mixed_case = r#"{"jSoNRPC":"2.0","id":123,"result":"OK"}"#;
        serde_json::from_str::<Response<String>>(mixed_case).unwrap();
    }

    /// Tests that unknown fields are ignored, and deserialize continues.
    /// Also that unicode and backslashes work.
    #[test]
    fn unknown_fields_and_unicode() {
        let e = ErrorObject::internal_error();
        let j = json!({
            "error": e,
            "\u{00f8}": 123,
            "id": 0,
            "unknown_field": 123,
            "jsonrpc": "2.0",
            "unknown_field": 123
        });
        let resp = serde_json::from_value::<Response<String>>(j).unwrap();
        assert_eq!(resp, Response::internal_error(Id::Num(0)));
    }
}

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
    #[inline]
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

    #[inline]
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

    #[inline]
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

    #[inline]
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

    #[inline]
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

    #[inline]
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

    #[inline]
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
    T: Clone + Serialize,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string_pretty(self) {
            Ok(json) => write!(f, "{json}"),
            Err(_) => Err(std::fmt::Error),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Key
/// TODO
pub(crate) enum Key {
    /// TODO
    JsonRpc,
    /// TODO
    Result,
    /// TODO
    Error,
    /// TODO
    Id,
}

/// TODO
struct KeyVisitor;

impl serde::de::Visitor<'_> for KeyVisitor {
    type Value = Key;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Key field must be a string and one of the following values: ['jsonrpc', 'result', 'error', 'id']")
    }

    fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Self::Value, E> {
        if text.eq_ignore_ascii_case("jsonrpc") {
            Ok(Key::JsonRpc)
        } else if text.eq_ignore_ascii_case("result") {
            Ok(Key::Result)
        } else if text.eq_ignore_ascii_case("error") {
            Ok(Key::Error)
        } else if text.eq_ignore_ascii_case("id") {
            Ok(Key::Id)
        } else {
            Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Str(text),
                &self,
            ))
        }
    }
}

//---------------------------------------------------------------------------------------------------- Serde impl
impl<'a> Deserialize<'a> for Key {
    fn deserialize<D: Deserializer<'a>>(des: D) -> Result<Self, D::Error> {
        des.deserialize_str(KeyVisitor)
    }
}

impl<T> Serialize for Response<T>
where
    T: Serialize + Clone,
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

impl<'de, T> Deserialize<'de> for Response<T>
where
    T: Clone + Deserialize<'de> + 'de,
{
    fn deserialize<D: Deserializer<'de>>(der: D) -> Result<Self, D::Error> {
        use core::marker::PhantomData;
        use serde::de::{self, Visitor};

        /// TODO
        struct MapVisit<T>(PhantomData<T>);

        impl<'de, T> Visitor<'de> for MapVisit<T>
        where
            T: Clone + Deserialize<'de> + 'de,
        {
            type Value = Response<T>;

            #[inline]
            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str("JSON-RPC 2.0 Response")
            }

            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut jsonrpc = None;
                let mut payload = None;
                let mut id = None;

                while let Some(key) = map.next_key::<Key>()? {
                    match key {
                        Key::JsonRpc => jsonrpc = Some(map.next_value::<Version>()?),

                        Key::Result => {
                            if payload.is_none() {
                                payload = Some(Ok(map.next_value::<T>()?));
                            } else {
                                return Err(serde::de::Error::duplicate_field(
                                    "both result and error found",
                                ));
                            }
                        }

                        Key::Error => {
                            if payload.is_none() {
                                payload = Some(Err(map.next_value::<ErrorObject>()?));
                            } else {
                                return Err(serde::de::Error::duplicate_field(
                                    "both result and error found",
                                ));
                            }
                        }

                        Key::Id => id = map.next_value::<Option<Id>>()?,
                    }
                }

                use serde::de::Error;

                let response = match (jsonrpc, id, payload) {
                    (Some(jsonrpc), Some(id), Some(payload)) => Response {
                        jsonrpc,
                        id,
                        payload,
                    },
                    (None, None, None) => {
                        return Err(Error::missing_field("jsonrpc + id + result/error"))
                    }
                    (None, _, _) => return Err(Error::missing_field("jsonrpc")),
                    (_, None, _) => return Err(Error::missing_field("id")),
                    (_, _, None) => return Err(Error::missing_field("result/error")),
                };

                Ok(response)
            }
        }

        /// TODO
        const FIELDS: &[&str] = &["jsonrpc", "payload", "id"];
        der.deserialize_struct("Response", FIELDS, MapVisit(PhantomData))
    }
}

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod test {
    use super::*;
    use crate::id::Id;

    /// Basic serde tests.
    #[test]
    fn serde() {
        let result = String::from("result_ok");
        let id = Id::Num(123);

        let req = Response::ok(id.clone(), result.clone());

        let ser: String = serde_json::to_string(&req).unwrap();
        let de: Response<String> = serde_json::from_str(&ser).unwrap();

        assert_eq!(de.payload.unwrap(), result);
        assert_eq!(de.id, id);
    }
}

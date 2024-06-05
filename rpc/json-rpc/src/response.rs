//! TODO

//---------------------------------------------------------------------------------------------------- Use
use crate::error::ErrorObject;
use crate::id::Id;
use crate::version::Version;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;

//---------------------------------------------------------------------------------------------------- Response
/// JSON-RPC 2.0 Response object
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response<'a, T> {
    /// TODO
    pub jsonrpc: Version,

    #[serde(borrow)]
    /// This field will always be serialized.
    ///
    /// Both in the case of `None` and `Some(Id::Null)`, it will be serialized as `"id": null`.
    pub id: Id<'a>,

    #[serde(borrow)]
    /// TODO
    pub payload: Result<T, ErrorObject<'a>>,
}

impl<'a, T> Response<'a, T> {
    #[inline]
    /// Creates a successful response.
    pub const fn ok(id: Id<'a>, payload: T) -> Self {
        Self {
            jsonrpc: Version,
            id,
            payload: Ok(payload),
        }
    }

    #[inline]
    /// Creates an error response.
    pub const fn err(id: Id<'a>, payload: ErrorObject<'a>) -> Self {
        Self {
            jsonrpc: Version,
            id,
            payload: Err(payload),
        }
    }

    #[inline]
    /// [`ErrorObject::parse_error`]
    pub const fn parse_error(id: Id<'a>) -> Self {
        Self {
            jsonrpc: Version,
            payload: Err(ErrorObject::parse_error()),
            id,
        }
    }

    #[inline]
    /// [`ErrorObject::invalid_request`]
    pub const fn invalid_request(id: Id<'a>) -> Self {
        Self {
            jsonrpc: Version,
            payload: Err(ErrorObject::invalid_request()),
            id,
        }
    }

    #[inline]
    /// [`ErrorObject::method_not_found`]
    pub const fn method_not_found(id: Id<'a>) -> Self {
        Self {
            jsonrpc: Version,
            payload: Err(ErrorObject::method_not_found()),
            id,
        }
    }

    #[inline]
    /// [`ErrorObject::invalid_params`]
    pub const fn invalid_params(id: Id<'a>) -> Self {
        Self {
            jsonrpc: Version,
            payload: Err(ErrorObject::invalid_params()),
            id,
        }
    }

    #[inline]
    /// [`ErrorObject::internal_error`]
    pub const fn internal_error(id: Id<'a>) -> Self {
        Self {
            jsonrpc: Version,
            payload: Err(ErrorObject::internal_error()),
            id,
        }
    }

    // #[inline]
    // /// [`UNKNOWN_ERROR`]
    // pub const fn unknown_error(id: Option<Id<'a>>) -> Self {
    //     Self {
    //         jsonrpc: Version,
    //         payload: Err(ErrorObject::unknown_error()),
    //         id,
    //     }
    // }

    // #[inline]
    // /// [`BATCH_NOT_SUPPORTED`]
    // pub const fn batch_not_supported(id: Option<Id<'a>>) -> Self {
    //     Self {
    //         jsonrpc: Version,
    //         payload: Err(ErrorObject::batch_not_supported()),
    //         id,
    //     }
    // }

    // #[inline]
    // /// [`OVERSIZED_REQUEST`]
    // pub const fn oversized_request(id: Option<Id<'a>>) -> Self {
    //     Self {
    //         jsonrpc: Version,
    //         payload: Err(ErrorObject::oversized_request()),
    //         id,
    //     }
    // }

    // #[inline]
    // /// [`OVERSIZED_RESPONSE`]
    // pub const fn oversized_response(id: Option<Id<'a>>) -> Self {
    //     Self {
    //         jsonrpc: Version,
    //         payload: Err(ErrorObject::oversized_response()),
    //         id,
    //     }
    // }

    // #[inline]
    // /// [`OVERSIZED_BATCH_REQUEST`]
    // pub const fn oversized_batch_request(id: Option<Id<'a>>) -> Self {
    //     Self {
    //         jsonrpc: Version,
    //         payload: Err(ErrorObject::oversized_batch_request()),
    //         id,
    //     }
    // }

    // #[inline]
    // /// [`OVERSIZED_BATCH_REQUEST`]
    // pub const fn oversized_batch_response(id: Option<Id<'a>>) -> Self {
    //     Self {
    //         jsonrpc: Version,
    //         payload: Err(ErrorObject::oversized_batch_response()),
    //         id,
    //     }
    // }

    // #[inline]
    // /// [`SERVER_IS_BUSY`]
    // pub const fn server_is_busy(id: Option<Id<'a>>) -> Self {
    //     Self {
    //         jsonrpc: Version,
    //         payload: Err(ErrorObject::server_is_busy()),
    //         id,
    //     }
    // }
}

//---------------------------------------------------------------------------------------------------- Trait impl
impl<T> std::fmt::Display for Response<'_, T>
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

// impl<T> Serialize for Response<T>
// where
//     T: Serialize + Clone,
// {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let mut s = serializer.serialize_struct("Response", 3)?;

//         s.serialize_field("jsonrpc", &self.jsonrpc)?;

//         // This member is required.
//         //
//         // Even if `null`, or the client `Request` didn't include one.
//         s.serialize_field("id", &self.id)?;

//         match &self.payload {
//             Ok(r) => s.serialize_field("result", r)?,
//             Err(e) => s.serialize_field("error", e)?,
//         }

//         s.end()
//     }
// }

// impl<'de: 'a, 'a, T> Deserialize<'de> for Response<T>
// where
//     T: Clone + Deserialize<'de> + 'de,
// {
//     fn deserialize<D: Deserializer<'de>>(der: D) -> Result<Self, D::Error> {
//         use core::marker::PhantomData;
//         use serde::de::{self, Visitor};

//         /// TODO
//         struct MapVisit<T>(PhantomData<T>);

//         impl<'de, T> Visitor<'de> for MapVisit<T>
//         where
//             T: Clone + Deserialize<'de> + 'de,
//         {
//             type Value = Response<'de, T>;

//             #[inline]
//             fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
//                 formatter.write_str("JSON-RPC 2.0 Response")
//             }

//             fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
//                 let mut jsonrpc = None;
//                 let mut payload = None;
//                 let mut id = None;

//                 use crate::key::Key;

//                 while let Some(key) = map.next_key::<Key>()? {
//                     match key {
//                         Key::JsonRpc => jsonrpc = Some(map.next_value::<Version>()?),

//                         Key::Result => {
//                             if payload.is_none() {
//                                 payload = Some(Ok(map.next_value::<Cow<'de, T>>()?));
//                             } else {
//                                 return Err(serde::de::Error::duplicate_field(
//                                     "both result and error found",
//                                 ));
//                             }
//                         }

//                         Key::Error => {
//                             if payload.is_none() {
//                                 payload = Some(Err(map.next_value::<ErrorObject<'de>>()?));
//                             } else {
//                                 return Err(serde::de::Error::duplicate_field(
//                                     "both result and error found",
//                                 ));
//                             }
//                         }

//                         Key::Id => id = map.next_value::<Option<Id<'de>>>()?,
//                     }
//                 }

//                 let response = match (jsonrpc, payload) {
//                     (Some(jsonrpc), Some(payload)) => Response {
//                         jsonrpc,
//                         id,
//                         payload,
//                     },
//                     (None, None) => {
//                         return Err(serde::de::Error::missing_field("jsonrpc + result/error"))
//                     }
//                     (None, _) => return Err(serde::de::Error::missing_field("jsonrpc")),
//                     (_, None) => return Err(serde::de::Error::missing_field("result/error")),
//                 };

//                 Ok(response)
//             }
//         }

//         /// TODO
//         const FIELDS: &[&str] = &["jsonrpc", "payload", "id"];
//         der.deserialize_struct("Response", FIELDS, MapVisit(PhantomData))
//     }
// }

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod test {
    use super::*;
    use crate::id::Id;

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

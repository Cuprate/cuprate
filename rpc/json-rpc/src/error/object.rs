//! TODO

//---------------------------------------------------------------------------------------------------- Use
use crate::error::{
    ErrorCode, BATCH_NOT_SUPPORTED, INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST,
    METHOD_NOT_FOUND, OVERSIZED_BATCH_REQUEST, OVERSIZED_BATCH_RESPONSE, OVERSIZED_REQUEST,
    OVERSIZED_RESPONSE, PARSE_ERROR, SERVER_IS_BUSY, UNKNOWN_ERROR,
};
use serde::{Deserialize, Serialize};
use serde_json::value::Value;
use std::borrow::Cow;

//---------------------------------------------------------------------------------------------------- ErrorObject
/// [Error object](https://www.jsonrpc.org/specification)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorObject<'a> {
    /// [`ErrorCode`]
    pub code: ErrorCode,

    #[serde(borrow)]
    /// Message
    pub message: Cow<'a, str>,

    #[serde(borrow)]
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional data
    pub data: Option<Cow<'a, Value>>,
}

impl ErrorObject<'_> {
    #[inline]
    /// Creates new error, deriving message from the code.
    pub const fn from_code(code: ErrorCode) -> Self {
        Self {
            code,
            message: Cow::Borrowed(code.msg()),
            data: None,
        }
    }

    #[inline]
    /// Convert `ErrorObject<'a>` to `ErrorObject<'static>`
    pub fn into_owned(self) -> ErrorObject<'static> {
        ErrorObject {
            code: self.code,
            message: Cow::Owned(self.message.into_owned()),
            data: self.data.map(|d| Cow::Owned(d.into_owned())),
        }
    }

    /// [`PARSE_ERROR`]
    pub const fn parse_error() -> Self {
        Self {
            code: ErrorCode::ServerError(PARSE_ERROR.0),
            message: Cow::Borrowed(PARSE_ERROR.1),
            data: None,
        }
    }

    /// [`INVALID_REQUEST`]
    pub const fn invalid_request() -> Self {
        Self {
            code: ErrorCode::ServerError(INVALID_REQUEST.0),
            message: Cow::Borrowed(INVALID_REQUEST.1),
            data: None,
        }
    }

    /// [`METHOD_NOT_FOUND`]
    pub const fn method_not_found() -> Self {
        Self {
            code: ErrorCode::ServerError(METHOD_NOT_FOUND.0),
            message: Cow::Borrowed(METHOD_NOT_FOUND.1),
            data: None,
        }
    }

    /// [`INVALID_PARAMS`]
    pub const fn invalid_params() -> Self {
        Self {
            code: ErrorCode::ServerError(INVALID_PARAMS.0),
            message: Cow::Borrowed(INVALID_PARAMS.1),
            data: None,
        }
    }

    /// [`INTERNAL_ERROR`]
    pub const fn internal_error() -> Self {
        Self {
            code: ErrorCode::ServerError(INTERNAL_ERROR.0),
            message: Cow::Borrowed(INTERNAL_ERROR.1),
            data: None,
        }
    }

    /// [`UNKNOWN_ERROR`]
    pub const fn unknown_error() -> Self {
        Self {
            code: ErrorCode::ServerError(UNKNOWN_ERROR.0),
            message: Cow::Borrowed(UNKNOWN_ERROR.1),
            data: None,
        }
    }

    /// [`BATCH_NOT_SUPPORTED`]
    pub const fn batch_not_supported() -> Self {
        Self {
            code: ErrorCode::ServerError(BATCH_NOT_SUPPORTED.0),
            message: Cow::Borrowed(BATCH_NOT_SUPPORTED.1),
            data: None,
        }
    }

    /// [`OVERSIZED_REQUEST`]
    pub const fn oversized_request() -> Self {
        Self {
            code: ErrorCode::ServerError(OVERSIZED_REQUEST.0),
            message: Cow::Borrowed(OVERSIZED_REQUEST.1),
            data: None,
        }
    }

    /// [`OVERSIZED_RESPONSE`]
    pub const fn oversized_response() -> Self {
        Self {
            code: ErrorCode::ServerError(OVERSIZED_RESPONSE.0),
            message: Cow::Borrowed(OVERSIZED_RESPONSE.1),
            data: None,
        }
    }

    /// [`OVERSIZED_BATCH_REQUEST`]
    pub const fn oversized_batch_request() -> Self {
        Self {
            code: ErrorCode::ServerError(OVERSIZED_BATCH_REQUEST.0),
            message: Cow::Borrowed(OVERSIZED_BATCH_REQUEST.1),
            data: None,
        }
    }

    /// [`OVERSIZED_BATCH_RESPONSE`]
    pub const fn oversized_batch_response() -> Self {
        Self {
            code: ErrorCode::ServerError(OVERSIZED_BATCH_RESPONSE.0),
            message: Cow::Borrowed(OVERSIZED_BATCH_RESPONSE.1),
            data: None,
        }
    }

    /// [`SERVER_IS_BUSY`]
    pub const fn server_is_busy() -> Self {
        Self {
            code: ErrorCode::ServerError(SERVER_IS_BUSY.0),
            message: Cow::Borrowed(SERVER_IS_BUSY.1),
            data: None,
        }
    }
}

//---------------------------------------------------------------------------------------------------- Trait impl
impl From<ErrorCode> for ErrorObject<'_> {
    fn from(code: ErrorCode) -> Self {
        Self::from_code(code)
    }
}

impl PartialEq for ErrorObject<'_> {
    fn eq(&self, other: &Self) -> bool {
        let this_v = self.data.as_ref();
        let other_v = other.data.as_ref();
        self.code == other.code && self.message == other.message && this_v == other_v
    }
}

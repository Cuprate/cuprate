//! TODO

//---------------------------------------------------------------------------------------------------- Use
use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use serde_json::value::Value;

use crate::error::{
    constants::{INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR},
    ErrorCode,
};

//---------------------------------------------------------------------------------------------------- ErrorObject
/// [Error object](https://www.jsonrpc.org/specification).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorObject {
    /// [`ErrorCode`]
    pub code: ErrorCode,

    /// Message
    pub message: Cow<'static, str>,

    /// Optional data
    pub data: Value,
}

impl ErrorObject {
    #[inline]
    /// Creates new error, deriving message from the code.
    pub const fn from_code(code: ErrorCode) -> Self {
        Self {
            code,
            message: Cow::Borrowed(code.msg()),
            data: Value::Null,
        }
    }

    /// [`PARSE_ERROR`]
    pub const fn parse_error() -> Self {
        Self {
            code: ErrorCode::ServerError(PARSE_ERROR.0),
            message: Cow::Borrowed(PARSE_ERROR.1),
            data: Value::Null,
        }
    }

    /// [`INVALID_REQUEST`]
    pub const fn invalid_request() -> Self {
        Self {
            code: ErrorCode::ServerError(INVALID_REQUEST.0),
            message: Cow::Borrowed(INVALID_REQUEST.1),
            data: Value::Null,
        }
    }

    /// [`METHOD_NOT_FOUND`]
    pub const fn method_not_found() -> Self {
        Self {
            code: ErrorCode::ServerError(METHOD_NOT_FOUND.0),
            message: Cow::Borrowed(METHOD_NOT_FOUND.1),
            data: Value::Null,
        }
    }

    /// [`INVALID_PARAMS`]
    pub const fn invalid_params() -> Self {
        Self {
            code: ErrorCode::ServerError(INVALID_PARAMS.0),
            message: Cow::Borrowed(INVALID_PARAMS.1),
            data: Value::Null,
        }
    }

    /// [`INTERNAL_ERROR`]
    pub const fn internal_error() -> Self {
        Self {
            code: ErrorCode::ServerError(INTERNAL_ERROR.0),
            message: Cow::Borrowed(INTERNAL_ERROR.1),
            data: Value::Null,
        }
    }

    // /// [`UNKNOWN_ERROR`]
    // pub const fn unknown_error() -> Self {
    //     Self {
    //         code: ErrorCode::ServerError(UNKNOWN_ERROR.0),
    //         message: Cow::Borrowed(UNKNOWN_ERROR.1),
    //         data: None,
    //     }
    // }

    // /// [`BATCH_NOT_SUPPORTED`]
    // pub const fn batch_not_supported() -> Self {
    //     Self {
    //         code: ErrorCode::ServerError(BATCH_NOT_SUPPORTED.0),
    //         message: Cow::Borrowed(BATCH_NOT_SUPPORTED.1),
    //         data: None,
    //     }
    // }

    // /// [`OVERSIZED_REQUEST`]
    // pub const fn oversized_request() -> Self {
    //     Self {
    //         code: ErrorCode::ServerError(OVERSIZED_REQUEST.0),
    //         message: Cow::Borrowed(OVERSIZED_REQUEST.1),
    //         data: None,
    //     }
    // }

    // /// [`OVERSIZED_RESPONSE`]
    // pub const fn oversized_response() -> Self {
    //     Self {
    //         code: ErrorCode::ServerError(OVERSIZED_RESPONSE.0),
    //         message: Cow::Borrowed(OVERSIZED_RESPONSE.1),
    //         data: None,
    //     }
    // }

    // /// [`OVERSIZED_BATCH_REQUEST`]
    // pub const fn oversized_batch_request() -> Self {
    //     Self {
    //         code: ErrorCode::ServerError(OVERSIZED_BATCH_REQUEST.0),
    //         message: Cow::Borrowed(OVERSIZED_BATCH_REQUEST.1),
    //         data: None,
    //     }
    // }

    // /// [`OVERSIZED_BATCH_RESPONSE`]
    // pub const fn oversized_batch_response() -> Self {
    //     Self {
    //         code: ErrorCode::ServerError(OVERSIZED_BATCH_RESPONSE.0),
    //         message: Cow::Borrowed(OVERSIZED_BATCH_RESPONSE.1),
    //         data: None,
    //     }
    // }

    // /// [`SERVER_IS_BUSY`]
    // pub const fn server_is_busy() -> Self {
    //     Self {
    //         code: ErrorCode::ServerError(SERVER_IS_BUSY.0),
    //         message: Cow::Borrowed(SERVER_IS_BUSY.1),
    //         data: None,
    //     }
    // }
}

//---------------------------------------------------------------------------------------------------- Trait impl
impl From<ErrorCode> for ErrorObject {
    fn from(code: ErrorCode) -> Self {
        Self::from_code(code)
    }
}

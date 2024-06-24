//! Error codes.

//---------------------------------------------------------------------------------------------------- Use
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::constants::{
    INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR, SERVER_ERROR,
};

//---------------------------------------------------------------------------------------------------- ErrorCode
/// [Error object code](https://www.jsonrpc.org/specification#error_object).
///
/// This `enum` encapsulates JSON-RPC 2.0's error codes
/// found in [`ErrorObject`](crate::error::ErrorObject).
///
/// It associates the code integer ([`i32`]) with its defined message.
///
/// # Application defined errors
/// The custom error codes past `-32099` (`-31000, -31001`, ...)
/// defined in JSON-RPC 2.0 are not supported by this enum because:
///
/// 1. The `(i32, &'static str)` required makes the enum more than 3x larger
/// 2. It is not used by Cuprate/Monero[^1]
///
/// [^1]: Defined errors used by Monero (also excludes the last defined error `-32000 to -32099 Server error`): <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/contrib/epee/include/net/http_server_handlers_map2.h#L150>
///
/// # Display
/// ```rust
/// use cuprate_json_rpc::error::ErrorCode;
/// use serde_json::{to_value, from_value, Value};
///
/// for e in [
///     ErrorCode::ParseError,
///     ErrorCode::InvalidRequest,
///     ErrorCode::MethodNotFound,
///     ErrorCode::InvalidParams,
///     ErrorCode::InternalError,
///     ErrorCode::ServerError(0),
/// ] {
///     // The formatting is `$CODE: $MSG`.
///     let expected_fmt = format!("{}: {}", e.code(), e.msg());
///     assert_eq!(expected_fmt, format!("{e}"));
/// }
/// ```
///
/// # (De)serialization
/// This type gets (de)serialized as the associated `i32`, for example:
/// ```rust
/// use cuprate_json_rpc::error::ErrorCode;
/// use serde_json::{to_value, from_value, Value};
///
/// for e in [
///     ErrorCode::ParseError,
///     ErrorCode::InvalidRequest,
///     ErrorCode::MethodNotFound,
///     ErrorCode::InvalidParams,
///     ErrorCode::InternalError,
///     ErrorCode::ServerError(0),
///     ErrorCode::ServerError(1),
///     ErrorCode::ServerError(2),
/// ] {
///     // Gets serialized into a JSON integer.
///     let value = to_value(&e).unwrap();
///     assert_eq!(value, Value::Number(e.code().into()));
///
///     // Expects a JSON integer when deserializing.
///     assert_eq!(e, from_value(value).unwrap());
/// }
/// ```
///
/// ```rust,should_panic
/// # use cuprate_json_rpc::error::ErrorCode;
/// # use serde_json::from_value;
/// // A JSON string that contains an integer won't work.
/// from_value::<ErrorCode>("-32700".into()).unwrap();
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, thiserror::Error)]
pub enum ErrorCode {
    #[error("{}: {}", PARSE_ERROR.0, PARSE_ERROR.1)]
    /// Invalid JSON was received by the server.
    ///
    /// An error occurred on the server while parsing the JSON text.
    ParseError,

    #[error("{}: {}", INVALID_REQUEST.0, INVALID_REQUEST.1)]
    /// The JSON sent is not a valid Request object.
    InvalidRequest,

    #[error("{}: {}", METHOD_NOT_FOUND.0, METHOD_NOT_FOUND.1)]
    /// The method does not exist / is not available.
    MethodNotFound,

    #[error("{}: {}", INVALID_PARAMS.0, INVALID_PARAMS.1)]
    /// Invalid method parameters.
    InvalidParams,

    #[error("{}: {}", INTERNAL_ERROR.0, INTERNAL_ERROR.1)]
    /// Internal JSON-RPC error.
    InternalError,

    #[error("{0}: {SERVER_ERROR}")]
    /// Reserved for implementation-defined server-errors.
    ServerError(i32),
}

impl ErrorCode {
    /// Creates [`Self`] from a [`i32`] code.
    ///
    /// [`From<i32>`] is the same as this function.
    ///
    /// ```rust
    /// use cuprate_json_rpc::error::{
    ///     ErrorCode,
    ///     INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR,
    /// };
    ///
    /// assert_eq!(ErrorCode::from_code(PARSE_ERROR.0),      ErrorCode::ParseError);
    /// assert_eq!(ErrorCode::from_code(INVALID_REQUEST.0),  ErrorCode::InvalidRequest);
    /// assert_eq!(ErrorCode::from_code(METHOD_NOT_FOUND.0), ErrorCode::MethodNotFound);
    /// assert_eq!(ErrorCode::from_code(INVALID_PARAMS.0),   ErrorCode::InvalidParams);
    /// assert_eq!(ErrorCode::from_code(INTERNAL_ERROR.0),   ErrorCode::InternalError);
    ///
    /// // Non-defined code inputs will default to a custom `ServerError`.
    /// assert_eq!(ErrorCode::from_code(0), ErrorCode::ServerError(0));
    /// assert_eq!(ErrorCode::from_code(1), ErrorCode::ServerError(1));
    /// assert_eq!(ErrorCode::from_code(2), ErrorCode::ServerError(2));
    /// ```
    pub const fn from_code(code: i32) -> Self {
        // FIXME: you cannot `match` on tuple fields
        // so use `if` (seems to compile to the same
        // assembly as matching directly on `i32`s).
        if code == PARSE_ERROR.0 {
            Self::ParseError
        } else if code == INVALID_REQUEST.0 {
            Self::InvalidRequest
        } else if code == METHOD_NOT_FOUND.0 {
            Self::MethodNotFound
        } else if code == INVALID_PARAMS.0 {
            Self::InvalidParams
        } else if code == INTERNAL_ERROR.0 {
            Self::InternalError
        } else {
            Self::ServerError(code)
        }
    }

    /// Returns `self`'s [`i32`] code representation.
    ///
    /// ```rust
    /// use cuprate_json_rpc::error::{
    ///     ErrorCode,
    ///     INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR,
    /// };
    ///
    /// assert_eq!(ErrorCode::ParseError.code(),     PARSE_ERROR.0);
    /// assert_eq!(ErrorCode::InvalidRequest.code(), INVALID_REQUEST.0);
    /// assert_eq!(ErrorCode::MethodNotFound.code(), METHOD_NOT_FOUND.0);
    /// assert_eq!(ErrorCode::InvalidParams.code(),  INVALID_PARAMS.0);
    /// assert_eq!(ErrorCode::InternalError.code(),  INTERNAL_ERROR.0);
    /// assert_eq!(ErrorCode::ServerError(0).code(), 0);
    /// assert_eq!(ErrorCode::ServerError(1).code(), 1);
    /// ```
    pub const fn code(&self) -> i32 {
        match self {
            Self::ParseError => PARSE_ERROR.0,
            Self::InvalidRequest => INVALID_REQUEST.0,
            Self::MethodNotFound => METHOD_NOT_FOUND.0,
            Self::InvalidParams => INVALID_PARAMS.0,
            Self::InternalError => INTERNAL_ERROR.0,
            Self::ServerError(code) => *code,
        }
    }

    /// Returns `self`'s human readable [`str`] message.
    ///
    /// ```rust
    /// use cuprate_json_rpc::error::{
    ///     ErrorCode,
    ///     INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR, SERVER_ERROR,
    /// };
    ///
    /// assert_eq!(ErrorCode::ParseError.msg(),     PARSE_ERROR.1);
    /// assert_eq!(ErrorCode::InvalidRequest.msg(), INVALID_REQUEST.1);
    /// assert_eq!(ErrorCode::MethodNotFound.msg(), METHOD_NOT_FOUND.1);
    /// assert_eq!(ErrorCode::InvalidParams.msg(),  INVALID_PARAMS.1);
    /// assert_eq!(ErrorCode::InternalError.msg(),  INTERNAL_ERROR.1);
    /// assert_eq!(ErrorCode::ServerError(0).msg(), SERVER_ERROR);
    /// ```
    pub const fn msg(&self) -> &'static str {
        match self {
            Self::ParseError => PARSE_ERROR.1,
            Self::InvalidRequest => INVALID_REQUEST.1,
            Self::MethodNotFound => METHOD_NOT_FOUND.1,
            Self::InvalidParams => INVALID_PARAMS.1,
            Self::InternalError => INTERNAL_ERROR.1,
            Self::ServerError(_) => SERVER_ERROR,
        }
    }
}

//---------------------------------------------------------------------------------------------------- Trait impl
impl<N: Into<i32>> From<N> for ErrorCode {
    fn from(code: N) -> Self {
        Self::from_code(code.into())
    }
}

//---------------------------------------------------------------------------------------------------- Serde impl
impl<'a> Deserialize<'a> for ErrorCode {
    fn deserialize<D: Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::from_code(Deserialize::deserialize(deserializer)?))
    }
}

impl Serialize for ErrorCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i32(self.code())
    }
}

//! Error object.

//---------------------------------------------------------------------------------------------------- Use
use std::{borrow::Cow, error::Error, fmt::Display};

use serde::{Deserialize, Serialize};
use serde_json::value::Value;

use crate::error::{
    constants::{
        INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR,
        SERVER_ERROR,
    },
    ErrorCode,
};

//---------------------------------------------------------------------------------------------------- ErrorObject
/// [The error object](https://www.jsonrpc.org/specification).
///
/// This is the object sent back in a [`Response`](crate::Response)
/// if the method call errored.
///
/// # Display
/// ```rust
/// use cuprate_json_rpc::error::ErrorObject;
///
/// // The format is `$CODE: $MESSAGE`.
/// // If a message was not passed during construction,
/// // the error code's message will be used.
/// assert_eq!(format!("{}", ErrorObject::parse_error()),      "-32700: Parse error");
/// assert_eq!(format!("{}", ErrorObject::invalid_request()),  "-32600: Invalid Request");
/// assert_eq!(format!("{}", ErrorObject::method_not_found()), "-32601: Method not found");
/// assert_eq!(format!("{}", ErrorObject::invalid_params()),   "-32602: Invalid params");
/// assert_eq!(format!("{}", ErrorObject::internal_error()),   "-32603: Internal error");
/// assert_eq!(format!("{}", ErrorObject::server_error(0)),    "0: Server error");
///
/// // Set a custom message.
/// let mut e = ErrorObject::server_error(1);
/// e.message = "hello".into();
/// assert_eq!(format!("{e}"), "1: hello");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorObject {
    /// The error code.
    pub code: ErrorCode,

    /// A custom message for this error, distinct from [`ErrorCode::msg`].
    ///
    /// A JSON `string` value.
    ///
    /// This is a `Cow<'static, str>` to support both 0-allocation for
    /// `const` string ID's commonly found in programs, as well as support
    /// for runtime [`String`]'s.
    pub message: Cow<'static, str>,

    /// Optional data associated with the error.
    ///
    /// # `None` vs `Some(Value::Null)`
    /// This field will be completely omitted during serialization if [`None`],
    /// however if it is `Some(Value::Null)`, it will be serialized as `"data": null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl ErrorObject {
    /// Creates a new error, deriving the message from the code.
    ///
    /// Same as `ErrorObject::from(ErrorCode)`.
    ///
    /// ```rust
    /// use std::borrow::Cow;
    /// use cuprate_json_rpc::error::{ErrorCode, ErrorObject};
    ///
    /// for code in [
    ///     ErrorCode::ParseError,
    ///     ErrorCode::InvalidRequest,
    ///     ErrorCode::MethodNotFound,
    ///     ErrorCode::InvalidParams,
    ///     ErrorCode::InternalError,
    ///     ErrorCode::ServerError(0),
    /// ] {
    ///     let object = ErrorObject::from_code(code);
    ///     assert_eq!(object, ErrorObject {
    ///         code,
    ///         message: Cow::Borrowed(code.msg()),
    ///         data: None,
    ///     });
    ///
    /// }
    /// ```
    pub const fn from_code(code: ErrorCode) -> Self {
        Self {
            code,
            message: Cow::Borrowed(code.msg()),
            data: None,
        }
    }

    /// Creates a new error using [`PARSE_ERROR`].
    ///
    /// ```rust
    /// use std::borrow::Cow;
    /// use cuprate_json_rpc::error::{ErrorCode, ErrorObject};
    ///
    /// let code = ErrorCode::ParseError;
    /// let object = ErrorObject::parse_error();
    /// assert_eq!(object, ErrorObject {
    ///     code,
    ///     message: Cow::Borrowed(code.msg()),
    ///     data: None,
    /// });
    /// ```
    pub const fn parse_error() -> Self {
        Self {
            code: ErrorCode::ParseError,
            message: Cow::Borrowed(PARSE_ERROR.1),
            data: None,
        }
    }

    /// Creates a new error using [`INVALID_REQUEST`].
    ///
    /// ```rust
    /// use std::borrow::Cow;
    /// use cuprate_json_rpc::error::{ErrorCode, ErrorObject};
    ///
    /// let code = ErrorCode::InvalidRequest;
    /// let object = ErrorObject::invalid_request();
    /// assert_eq!(object, ErrorObject {
    ///     code,
    ///     message: Cow::Borrowed(code.msg()),
    ///     data: None,
    /// });
    /// ```
    pub const fn invalid_request() -> Self {
        Self {
            code: ErrorCode::InvalidRequest,
            message: Cow::Borrowed(INVALID_REQUEST.1),
            data: None,
        }
    }

    /// Creates a new error using [`METHOD_NOT_FOUND`].
    ///
    /// ```rust
    /// use std::borrow::Cow;
    /// use cuprate_json_rpc::error::{ErrorCode, ErrorObject};
    ///
    /// let code = ErrorCode::MethodNotFound;
    /// let object = ErrorObject::method_not_found();
    /// assert_eq!(object, ErrorObject {
    ///     code,
    ///     message: Cow::Borrowed(code.msg()),
    ///     data: None,
    /// });
    /// ```
    pub const fn method_not_found() -> Self {
        Self {
            code: ErrorCode::MethodNotFound,
            message: Cow::Borrowed(METHOD_NOT_FOUND.1),
            data: None,
        }
    }

    /// Creates a new error using [`INVALID_PARAMS`].
    ///
    /// ```rust
    /// use std::borrow::Cow;
    /// use cuprate_json_rpc::error::{ErrorCode, ErrorObject};
    ///
    /// let code = ErrorCode::InvalidParams;
    /// let object = ErrorObject::invalid_params();
    /// assert_eq!(object, ErrorObject {
    ///     code,
    ///     message: Cow::Borrowed(code.msg()),
    ///     data: None,
    /// });
    /// ```
    pub const fn invalid_params() -> Self {
        Self {
            code: ErrorCode::InvalidParams,
            message: Cow::Borrowed(INVALID_PARAMS.1),
            data: None,
        }
    }

    /// Creates a new error using [`INTERNAL_ERROR`].
    ///
    ///
    /// ```rust
    /// use std::borrow::Cow;
    /// use cuprate_json_rpc::error::{ErrorCode, ErrorObject};
    ///
    /// let code = ErrorCode::InternalError;
    /// let object = ErrorObject::internal_error();
    /// assert_eq!(object, ErrorObject {
    ///     code,
    ///     message: Cow::Borrowed(code.msg()),
    ///     data: None,
    /// });
    /// ```
    pub const fn internal_error() -> Self {
        Self {
            code: ErrorCode::InternalError,
            message: Cow::Borrowed(INTERNAL_ERROR.1),
            data: None,
        }
    }

    /// Creates a new error using [`SERVER_ERROR`].
    ///
    /// You must provide the custom [`i32`] error code.
    ///
    /// ```rust
    /// use std::borrow::Cow;
    /// use cuprate_json_rpc::error::{ErrorCode, ErrorObject};
    ///
    /// let code = ErrorCode::ServerError(0);
    /// let object = ErrorObject::server_error(0);
    /// assert_eq!(object, ErrorObject {
    ///     code,
    ///     message: Cow::Borrowed(code.msg()),
    ///     data: None,
    /// });
    /// ```
    pub const fn server_error(error_code: i32) -> Self {
        Self {
            code: ErrorCode::ServerError(error_code),
            message: Cow::Borrowed(SERVER_ERROR),
            data: None,
        }
    }
}

//---------------------------------------------------------------------------------------------------- Trait impl
impl From<ErrorCode> for ErrorObject {
    fn from(code: ErrorCode) -> Self {
        Self::from_code(code)
    }
}

impl Display for ErrorObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Using `self.code`'s formatting will write the
        // message twice, so prefer the built-in message.
        write!(f, "{}: {}", self.code.code(), self.message)
    }
}

impl Error for ErrorObject {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.code)
    }

    fn description(&self) -> &str {
        &self.message
    }
}

//---------------------------------------------------------------------------------------------------- Use
use serde::{Deserialize, Deserializer, Serialize, Serializer};

//---------------------------------------------------------------------------------------------------- Constants
#[rustfmt::skip]
mod constants {
   // [`JSON-RPC 2.0`](https://www.jsonrpc.org/specification#error_object) defined errors.
   pub const PARSE_ERROR:      (i32, &str) = (-32700, "Parse error");
   pub const INVALID_REQUEST:  (i32, &str) = (-32600, "Invalid Request");
   pub const METHOD_NOT_FOUND: (i32, &str) = (-32601, "Method not found");
   pub const INVALID_PARAMS:   (i32, &str) = (-32602, "Invalid params");
   pub const INTERNAL_ERROR:   (i32, &str) = (-32603, "Internal error");
   // Server-defined error.
   //
   // The [`i32`] error code is the caller's choice.
   pub const SERVER_ERROR: &str = "Server error";

   // These exist because when `match`'ing, you cannot
   // do `TUPLE_CONST.0`, see [`ErrorCode::from_code`] below.
   pub(super) const PARSE_ERROR_I32:      i32 = PARSE_ERROR.0;
   pub(super) const INVALID_REQUEST_I32:  i32 = INVALID_REQUEST.0;
   pub(super) const METHOD_NOT_FOUND_I32: i32 = METHOD_NOT_FOUND.0;
   pub(super) const INVALID_PARAMS_I32:   i32 = INVALID_PARAMS.0;
   pub(super) const INTERNAL_ERROR_I32:   i32 = INTERNAL_ERROR.0;

   // Common custom errors.
   pub const UNKNOWN_ERROR:            (i32, &str) = (-32000, "Unknown error");
   pub const BATCH_NOT_SUPPORTED:      (i32, &str) = (-32001, "Batched requests are not supported by this server");
   pub const LIMIT_REQUEST:            (i32, &str) = (-32002, "Request limit exceeded");
   pub const LIMIT_RESPONSE:           (i32, &str) = (-32003, "Response limit exceeded");
   pub const LIMIT_BATCH_REQUEST:      (i32, &str) = (-32004, "Batch request limit exceeded");
   pub const LIMIT_BATCH_RESPONSE:     (i32, &str) = (-32005, "Batch response limit exceeded");
   pub const OVERSIZED_REQUEST:        (i32, &str) = (-32006, "Request is too big");
   pub const OVERSIZED_RESPONSE:       (i32, &str) = (-32007, "Response is too big");
   pub const OVERSIZED_BATCH_REQUEST:  (i32, &str) = (-32008, "The batch request was too large");
   pub const OVERSIZED_BATCH_RESPONSE: (i32, &str) = (-32009, "The batch request was too large");
   pub const SERVER_IS_BUSY:           (i32, &str) = (-32010, "Server is busy");
}
pub use constants::*;

//---------------------------------------------------------------------------------------------------- ErrorCode
/// [5.1 Error object code](https://www.jsonrpc.org/specification)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, thiserror::Error)]
pub enum ErrorCode {
    #[error("{}", PARSE_ERROR.1)]
    /// Invalid JSON was received by the server.
    ///
    /// An error occurred on the server while parsing the JSON text.
    ParseError,
    #[error("{}", INVALID_REQUEST.1)]
    /// The JSON sent is not a valid Request object.
    InvalidRequest,
    #[error("{}", METHOD_NOT_FOUND.1)]
    /// The method does not exist / is not available.
    MethodNotFound,
    #[error("{}", INVALID_PARAMS.1)]
    /// Invalid method parameters.
    InvalidParams,
    #[error("{}", INTERNAL_ERROR.1)]
    /// Internal JSON-RPC error.
    InternalError,
    #[error("{SERVER_ERROR} {0}")]
    /// Reserved for implementation-defined server-errors.
    ServerError(i32),
}

impl ErrorCode {
    /// Creates [`Self`] from a code.
    pub const fn from_code(code: i32) -> Self {
        match code {
            PARSE_ERROR_I32 => ErrorCode::ParseError,
            INVALID_REQUEST_I32 => ErrorCode::InvalidRequest,
            METHOD_NOT_FOUND_I32 => ErrorCode::MethodNotFound,
            INVALID_PARAMS_I32 => ErrorCode::InvalidParams,
            INTERNAL_ERROR_I32 => ErrorCode::InternalError,
            code => ErrorCode::ServerError(code),
        }
    }

    /// Returns [`i32`] representation.
    pub const fn code(&self) -> i32 {
        match self {
            ErrorCode::ParseError => PARSE_ERROR.0,
            ErrorCode::InvalidRequest => INVALID_REQUEST.0,
            ErrorCode::MethodNotFound => METHOD_NOT_FOUND.0,
            ErrorCode::InvalidParams => INVALID_PARAMS.0,
            ErrorCode::InternalError => INTERNAL_ERROR.0,
            ErrorCode::ServerError(code) => *code,
        }
    }

    /// Returns human readable `str` version.
    pub const fn msg(&self) -> &'static str {
        match self {
            ErrorCode::ParseError => PARSE_ERROR.1,
            ErrorCode::InvalidRequest => INVALID_REQUEST.1,
            ErrorCode::MethodNotFound => METHOD_NOT_FOUND.1,
            ErrorCode::InvalidParams => INVALID_PARAMS.1,
            ErrorCode::InternalError => INTERNAL_ERROR.1,
            ErrorCode::ServerError(_) => SERVER_ERROR,
        }
    }
}

//---------------------------------------------------------------------------------------------------- Trait impl
// Implements `From<N>` where N is any number that can fit inside `i32`.
macro_rules! impl_from_num {
    ($($num:ty),* $(,)?) => {
        $(
            impl From<$num> for ErrorCode {
                #[inline]
                fn from(code: $num) -> ErrorCode {
                    Self::from_code(code as i32)
                }
            }
            impl From<&$num> for ErrorCode {
                #[inline]
                fn from(code: &$num) -> ErrorCode {
                    Self::from_code(*code as i32)
                }
            }
        )*
    };
}
impl_from_num!(i8, i16, i32, u8, u16);

impl<'a> Deserialize<'a> for ErrorCode {
    fn deserialize<D: Deserializer<'a>>(deserializer: D) -> Result<ErrorCode, D::Error> {
        Ok(ErrorCode::from_code(Deserialize::deserialize(
            deserializer,
        )?))
    }
}

impl Serialize for ErrorCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_i32(self.code())
    }
}

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // Tests if constants being converted are correct.
    fn convert() {
        for i in [
            PARSE_ERROR,
            INVALID_REQUEST,
            METHOD_NOT_FOUND,
            INVALID_PARAMS,
            INTERNAL_ERROR,
        ] {
            let err = ErrorCode::from_code(i.0);
            let msg = err.to_string();
            assert_eq!(err.code(), i.0);
            assert_eq!(err.msg(), i.1);
            assert_eq!(err.msg(), msg);
        }
    }

    #[test]
    // Tests custom server error works.
    fn server_error() {
        let code = -32000;

        let err = ErrorCode::ServerError(code);
        assert_eq!(err.code(), code);
        assert_eq!(format!("Server error {code}"), err.to_string());
    }
}

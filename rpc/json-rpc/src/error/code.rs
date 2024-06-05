//! TODO

//---------------------------------------------------------------------------------------------------- Use
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::constants::{
    INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR, SERVER_ERROR,
};

//---------------------------------------------------------------------------------------------------- Constants

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
        /// HACK: you cannot `match` on tuple fields
        /// like `PARSE_ERROR.0 => /*...*/` so extract
        /// it out here.
        #[allow(clippy::wildcard_imports, clippy::missing_docs_in_private_items)]
        mod i32s {
            use super::*;
            pub(super) const PARSE_ERROR_I32: i32 = PARSE_ERROR.0;
            pub(super) const INVALID_REQUEST_I32: i32 = INVALID_REQUEST.0;
            pub(super) const METHOD_NOT_FOUND_I32: i32 = METHOD_NOT_FOUND.0;
            pub(super) const INVALID_PARAMS_I32: i32 = INVALID_PARAMS.0;
            pub(super) const INTERNAL_ERROR_I32: i32 = INTERNAL_ERROR.0;
        }
        #[allow(clippy::wildcard_imports)]
        use i32s::*;

        match code {
            PARSE_ERROR_I32 => Self::ParseError,
            INVALID_REQUEST_I32 => Self::InvalidRequest,
            METHOD_NOT_FOUND_I32 => Self::MethodNotFound,
            INVALID_PARAMS_I32 => Self::InvalidParams,
            INTERNAL_ERROR_I32 => Self::InternalError,
            code => Self::ServerError(code),
        }
    }

    /// Returns [`i32`] representation.
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

    /// Returns human readable `str` version.
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
/// Implements `From<N>` where N is any number that can fit inside `i32`.
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
    fn deserialize<D: Deserializer<'a>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::from_code(Deserialize::deserialize(deserializer)?))
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

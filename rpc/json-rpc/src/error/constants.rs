//! [`JSON-RPC 2.0`](https://www.jsonrpc.org/specification#error_object) defined errors as constants.

//---------------------------------------------------------------------------------------------------- Use

//---------------------------------------------------------------------------------------------------- JSON-RPC spec errors.
/// Code and message for [`ErrorCode::ParseError`](crate::error::ErrorCode::ParseError).
pub const PARSE_ERROR: (i32, &str) = (-32700, "Parse error");

/// Code and message for [`ErrorCode::InvalidRequest`](crate::error::ErrorCode::InvalidRequest).
pub const INVALID_REQUEST: (i32, &str) = (-32600, "Invalid Request");

/// Code and message for [`ErrorCode::MethodNotFound`](crate::error::ErrorCode::MethodNotFound).
pub const METHOD_NOT_FOUND: (i32, &str) = (-32601, "Method not found");

/// Code and message for [`ErrorCode::InvalidParams`](crate::error::ErrorCode::InvalidParams).
pub const INVALID_PARAMS: (i32, &str) = (-32602, "Invalid params");

/// Code and message for [`ErrorCode::InternalError`](crate::error::ErrorCode::InternalError).
pub const INTERNAL_ERROR: (i32, &str) = (-32603, "Internal error");

/// Message for [`ErrorCode::ServerError`](crate::error::ErrorCode::ServerError).
///
/// The [`i32`] error code is the caller's choice, this is only the message.
pub const SERVER_ERROR: &str = "Server error";

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod tests {
    // use super::*;
}

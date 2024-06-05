//! [`JSON-RPC 2.0`](https://www.jsonrpc.org/specification#error_object) defined errors.
//!
//! TODO

//---------------------------------------------------------------------------------------------------- Use

//---------------------------------------------------------------------------------------------------- JSON-RPC spec errors.
/// TODO
pub const PARSE_ERROR: (i32, &str) = (-32700, "Parse error");

/// TODO
pub const INVALID_REQUEST: (i32, &str) = (-32600, "Invalid Request");

/// TODO
pub const METHOD_NOT_FOUND: (i32, &str) = (-32601, "Method not found");

/// TODO
pub const INVALID_PARAMS: (i32, &str) = (-32602, "Invalid params");

/// TODO
pub const INTERNAL_ERROR: (i32, &str) = (-32603, "Internal error");

/// Server-defined error.
///
/// The [`i32`] error code is the caller's choice.
pub const SERVER_ERROR: &str = "Server error";

// // Common custom errors.
// pub const UNKNOWN_ERROR: (i32, &str) = (-32000, "Unknown error");
// pub const BATCH_NOT_SUPPORTED: (i32, &str) =
//     (-32001, "Batched requests are not supported by this server");
// pub const LIMIT_REQUEST: (i32, &str) = (-32002, "Request limit exceeded");
// pub const LIMIT_RESPONSE: (i32, &str) = (-32003, "Response limit exceeded");
// pub const LIMIT_BATCH_REQUEST: (i32, &str) = (-32004, "Batch request limit exceeded");
// pub const LIMIT_BATCH_RESPONSE: (i32, &str) = (-32005, "Batch response limit exceeded");
// pub const OVERSIZED_REQUEST: (i32, &str) = (-32006, "Request is too big");
// pub const OVERSIZED_RESPONSE: (i32, &str) = (-32007, "Response is too big");
// pub const OVERSIZED_BATCH_REQUEST: (i32, &str) = (-32008, "The batch request was too large");
// pub const OVERSIZED_BATCH_RESPONSE: (i32, &str) = (-32009, "The batch request was too large");
// pub const SERVER_IS_BUSY: (i32, &str) = (-32010, "Server is busy");

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod tests {
    // use super::*;
}

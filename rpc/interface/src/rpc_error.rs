//! RPC errors.

//---------------------------------------------------------------------------------------------------- Import
use axum::http::StatusCode;

//---------------------------------------------------------------------------------------------------- RpcError
/// Possible errors during RPC operation.
///
/// These are any errors that can happen _during_ a handler function.
/// I.e. if this error surfaces, it happened _after_ the request was
/// deserialized.
///
/// This is the `Error` type required to be used in an [`RpcHandler`](crate::RpcHandler).
///
/// TODO: This is empty as possible errors will be
/// enumerated when the handler functions are created.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RpcError {}

impl From<RpcError> for StatusCode {
    fn from(value: RpcError) -> Self {
        // TODO
        Self::INTERNAL_SERVER_ERROR
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

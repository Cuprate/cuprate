//! RPC errors.

//---------------------------------------------------------------------------------------------------- Import
use axum::http::StatusCode;

use cuprate_database::RuntimeError;

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
#[derive(Debug, thiserror::Error)]
pub enum RpcError {
    /// A [`std::io::Error`] from the database.
    #[error("database I/O error: {0}")]
    DatabaseIo(#[from] std::io::Error),

    /// A (non-I/O related) database error.
    #[error("database error: {0}")]
    DatabaseError(RuntimeError),
}

impl From<RpcError> for StatusCode {
    fn from(_: RpcError) -> Self {
        // TODO
        Self::INTERNAL_SERVER_ERROR
    }
}

impl From<RuntimeError> for RpcError {
    fn from(error: RuntimeError) -> Self {
        match error {
            RuntimeError::Io(io) => Self::DatabaseIo(io),
            RuntimeError::KeyExists
            | RuntimeError::KeyNotFound
            | RuntimeError::ResizeNeeded
            | RuntimeError::TableNotFound => Self::DatabaseError(error),
        }
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

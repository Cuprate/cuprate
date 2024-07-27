//! TODO

//---------------------------------------------------------------------------------------------------- Import
use axum::http::StatusCode;

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
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

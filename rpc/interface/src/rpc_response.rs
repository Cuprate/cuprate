//! RPC responses.

//---------------------------------------------------------------------------------------------------- Import
use cuprate_rpc_types::{bin::BinResponse, json::JsonRpcResponse, other::OtherResponse};

//---------------------------------------------------------------------------------------------------- RpcResponse
/// All possible RPC responses.
///
/// This enum encapsulates all possible RPC responses:
/// - JSON RPC 2.0 responses
/// - Binary responses
/// - Other JSON responses
///
/// It is the `Response` type required to be used in an [`RpcHandler`](crate::RpcHandler).
pub enum RpcResponse {
    /// JSON RPC 2.0 responses.
    JsonRpc(cuprate_json_rpc::Response<JsonRpcResponse>),
    /// Binary responses.
    Binary(BinResponse),
    /// Other JSON responses.
    Other(OtherResponse),
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

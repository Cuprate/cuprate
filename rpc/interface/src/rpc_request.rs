//! RPC requests.

//---------------------------------------------------------------------------------------------------- Import
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use cuprate_rpc_types::{bin::BinRequest, json::JsonRpcRequest, other::OtherRequest};

//---------------------------------------------------------------------------------------------------- RpcRequest
/// All possible RPC requests.
///
/// This enum encapsulates all possible RPC requests:
/// - JSON RPC 2.0 requests
/// - Binary requests
/// - Other JSON requests
///
/// It is the `Request` type required to be used in an [`RpcHandler`](crate::RpcHandler).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum RpcRequest {
    /// JSON-RPC 2.0 requests.
    JsonRpc(cuprate_json_rpc::Request<JsonRpcRequest>),
    /// Binary requests.
    Binary(BinRequest),
    /// Other JSON requests.
    Other(OtherRequest),
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

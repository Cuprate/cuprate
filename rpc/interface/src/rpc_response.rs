//! RPC responses.

//---------------------------------------------------------------------------------------------------- Import
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

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
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[allow(clippy::large_enum_variant)] // FIXME: maybe don't wrap internally with an enum?
pub enum RpcResponse {
    /// JSON RPC 2.0 responses.
    JsonRpc(JsonRpcResponse),
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

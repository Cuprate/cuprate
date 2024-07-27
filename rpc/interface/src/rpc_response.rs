//! TODO

//---------------------------------------------------------------------------------------------------- Import
use cuprate_rpc_types::{bin::BinResponse, json::JsonRpcResponse, other::OtherResponse};

//---------------------------------------------------------------------------------------------------- Status
/// TODO
pub enum RpcResponse {
    /// TODO
    JsonRpc(cuprate_json_rpc::Response<JsonRpcResponse>),
    /// TODO
    Binary(BinResponse),
    /// TODO
    Other(OtherResponse),
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

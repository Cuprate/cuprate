//! TODO

//---------------------------------------------------------------------------------------------------- Import
use cuprate_rpc_types::{bin::BinRequest, json::JsonRpcRequest, other::OtherRequest};

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
pub enum RpcRequest {
    /// TODO
    JsonRpc(cuprate_json_rpc::Request<JsonRpcRequest>),
    /// TODO
    Binary(BinRequest),
    /// TODO
    Other(OtherRequest),
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}

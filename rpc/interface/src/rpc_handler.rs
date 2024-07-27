//! TODO

//---------------------------------------------------------------------------------------------------- Use
use std::task::Poll;

use futures::{channel::oneshot::channel, FutureExt};
use tower::Service;

use cuprate_helper::asynch::InfallibleOneshotReceiver;
use cuprate_json_rpc::Id;
use cuprate_rpc_types::json::JsonRpcRequest;

use crate::{rpc_error::RpcError, rpc_request::RpcRequest, rpc_response::RpcResponse};

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
pub trait RpcHandler:
    Clone
    + Send
    + Sync
    + 'static
    + Service<
        RpcRequest,
        Response = RpcResponse,
        Error = RpcError,
        Future = InfallibleOneshotReceiver<Result<RpcResponse, RpcError>>,
    >
{
    /// TODO
    fn restricted(&self) -> bool;
}

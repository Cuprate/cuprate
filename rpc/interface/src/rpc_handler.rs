//! TODO

//---------------------------------------------------------------------------------------------------- Use
use std::task::Poll;

use futures::{channel::oneshot::channel, FutureExt};
use tower::Service;

use cuprate_helper::asynch::InfallibleOneshotReceiver;
use cuprate_json_rpc::Id;
use cuprate_rpc_types::json::JsonRpcRequest;

use crate::{error::Error, request::Request, response::Response};

//---------------------------------------------------------------------------------------------------- TODO
/// TODO
pub trait RpcHandler:
    Clone
    + Send
    + Sync
    + 'static
    + Service<
        Request,
        Response = Response,
        Error = Error,
        Future = InfallibleOneshotReceiver<Result<Response, Error>>,
    >
{
    /// TODO
    fn restricted(&self) -> bool;
}

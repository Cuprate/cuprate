//! RPC handler trait.

//---------------------------------------------------------------------------------------------------- Use
use std::future::Future;

use cuprate_rpc_types::{
    bin::{BinRequest, BinResponse},
    json::{JsonRpcRequest, JsonRpcResponse},
    other::{OtherRequest, OtherResponse},
};
use tower::Service;

use crate::{rpc_error::RpcError, rpc_request::RpcRequest, rpc_response::RpcResponse};

//---------------------------------------------------------------------------------------------------- RpcHandler
/// An RPC handler.
///
/// This trait represents a type that can turn [`RpcRequest`]s into [`RpcResponse`]s.
///
/// Implementors of this trait must be [`tower::Service`]s that use:
/// - [`RpcRequest`] as the generic `Request` type
/// - [`RpcResponse`] as the associated `Response` type
/// - [`RpcError`] as the associated `Error` type
/// - A generic [`Future`] that outputs `Result<RpcResponse, RpcError>`
///
/// See this crate's `RpcHandlerDummy` for an implementation example of this trait.
///
/// # Panics
/// Your [`RpcHandler`] must reply to [`RpcRequest`]s with the correct
/// [`RpcResponse`] or else this crate will panic during routing functions.
///
/// For example, upon a [`RpcRequest::Binary`] must be replied with
/// [`RpcRequest::Binary`]. If an [`RpcRequest::Other`] were returned instead,
/// this crate would panic.
pub trait RpcHandler:
    Clone
    + Send
    + Sync
    + 'static
    + Service<
        JsonRpcRequest,
        Response = JsonRpcResponse,
        Error = RpcError,
        Future: Future<Output = Result<JsonRpcResponse, RpcError>> + Send + Sync + 'static,
    >
    + Service<
        OtherRequest,
        Response = OtherResponse,
        Error = RpcError,
        Future: Future<Output = Result<OtherResponse, RpcError>> + Send + Sync + 'static,
    >
    + Service<
        BinRequest,
        Response = BinResponse,
        Error = RpcError,
        Future: Future<Output = Result<BinResponse, RpcError>> + Send + Sync + 'static,
    >
{
    /// Is this [`RpcHandler`] restricted?
    ///
    /// If this returns `true`, restricted methods and endpoints such as:
    /// - `/json_rpc`'s `relay_tx` method
    /// - The `/pop_blocks` endpoint
    ///
    /// will automatically be denied access when using the
    /// [`axum::Router`] provided by [`RouterBuilder`](crate::RouterBuilder).
    fn restricted(&self) -> bool;
}

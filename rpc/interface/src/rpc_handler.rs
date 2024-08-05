//! RPC handler trait.

//---------------------------------------------------------------------------------------------------- Use
use std::{future::Future, task::Poll};

use axum::{http::StatusCode, response::IntoResponse};
use futures::{channel::oneshot::channel, FutureExt};
use tower::Service;

use cuprate_helper::asynch::InfallibleOneshotReceiver;
use cuprate_json_rpc::Id;
use cuprate_rpc_types::json::JsonRpcRequest;

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
/// - A generic [`Future`] that outputs `Result<RpcResponse, RpcError>` and matches [`RpcHandler::Future2`]
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
    + Service<RpcRequest, Response = RpcResponse, Error = RpcError, Future = Self::Future2>
{
    /// This is the [`Future`] your [`RpcHandler`] outputs.
    ///
    /// It is used as the type for [`tower::Service::Future`].
    ///
    /// For example, `RpcHandlerDummy`'s [`RpcHandler::Future2`] is:
    /// - `InfallibleOneshotReceiver<Result<RpcResponse, RpcError>>`
    type Future2: Future<Output = Result<RpcResponse, RpcError>> + Send + Sync + 'static;

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

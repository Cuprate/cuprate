//! RPC handler trait.

//---------------------------------------------------------------------------------------------------- Use
use cuprate_rpc_types::{
    bin::{BinRequest, BinResponse},
    json::{JsonRpcRequest, JsonRpcResponse},
    other::{OtherRequest, OtherResponse},
};

use crate::RpcService;

//---------------------------------------------------------------------------------------------------- RpcHandler
/// An RPC handler.
///
/// This trait represents a type that can turn `Request`s into `Response`s.
///
/// Implementors of this trait must be:
/// - A [`tower::Service`]s that use [`JsonRpcRequest`] & [`JsonRpcResponse`]
/// - A [`tower::Service`]s that use [`BinRequest`] & [`BinResponse`]
/// - A [`tower::Service`]s that use [`OtherRequest`] & [`OtherResponse`]
///
/// In other words, an [`RpcHandler`] is a type that implements [`tower::Service`] 3 times,
/// one for each endpoint enum type found in [`cuprate_rpc_types`].
///
/// The error type must always be [`RpcError`](crate::RpcError).
///
/// See this crate's `RpcHandlerDummy` for an implementation example of this trait.
///
/// # Panics
/// Your [`RpcHandler`] must reply to `Request`s with the correct
/// `Response` or else this crate will panic during routing functions.
///
/// For example, upon a [`JsonRpcRequest::GetBlockCount`] must be replied with
/// [`JsonRpcResponse::GetBlockCount`]. If anything else is returned,
/// this crate may panic.
pub trait RpcHandler:
    RpcService<JsonRpcRequest, JsonRpcResponse>
    + RpcService<BinRequest, BinResponse>
    + RpcService<OtherRequest, OtherResponse>
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

//! Dummy implementation of [`RpcHandler`].

//---------------------------------------------------------------------------------------------------- Use
use std::task::Poll;

use cuprate_rpc_types::{
    bin::{BinRequest, BinResponse},
    json::{JsonRpcRequest, JsonRpcResponse},
    other::{OtherRequest, OtherResponse},
};
use futures::channel::oneshot::channel;
use serde::{Deserialize, Serialize};
use tower::Service;

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_helper::asynch::InfallibleOneshotReceiver;
use cuprate_json_rpc::Id;
use cuprate_rpc_interface::{RpcError, RpcHandler, RpcRequest, RpcResponse};
use cuprate_txpool::service::TxpoolReadHandle;

use crate::rpc::{bin, json, other};

//---------------------------------------------------------------------------------------------------- CupratedRpcHandler
/// TODO
#[derive(Clone)]
pub struct CupratedRpcHandler {
    /// Should this RPC server be [restricted](RpcHandler::restricted)?
    pub restricted: bool,

    /// Read handle to the blockchain database.
    pub blockchain: BlockchainReadHandle,

    /// Read handle to the transaction pool database.
    pub txpool: TxpoolReadHandle,
}

//---------------------------------------------------------------------------------------------------- RpcHandler Impl
impl RpcHandler for CupratedRpcHandler {
    fn restricted(&self) -> bool {
        self.restricted
    }
}

// INVARIANT:
//
// We don't need to check for `self.is_restricted()`
// here because `cuprate-rpc-interface` handles that.
//
// We can assume the request coming has the required permissions.

impl Service<JsonRpcRequest> for CupratedRpcHandler {
    type Response = JsonRpcResponse;
    type Error = RpcError;
    type Future = InfallibleOneshotReceiver<Result<JsonRpcResponse, RpcError>>;

    fn poll_ready(&mut self, _: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: JsonRpcRequest) -> Self::Future {
        let state = Self::clone(self);
        let response = json::map_request(state, request).expect("TODO");
        todo!()
    }
}

impl Service<BinRequest> for CupratedRpcHandler {
    type Response = BinResponse;
    type Error = RpcError;
    type Future = InfallibleOneshotReceiver<Result<BinResponse, RpcError>>;

    fn poll_ready(&mut self, _: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: BinRequest) -> Self::Future {
        let state = Self::clone(self);
        let response = bin::map_request(state, request).expect("TODO");
        todo!()
    }
}

impl Service<OtherRequest> for CupratedRpcHandler {
    type Response = OtherResponse;
    type Error = RpcError;
    type Future = InfallibleOneshotReceiver<Result<OtherResponse, RpcError>>;

    fn poll_ready(&mut self, _: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: OtherRequest) -> Self::Future {
        let state = Self::clone(self);
        let response = other::map_request(state, request).expect("TODO");
        todo!()
    }
}

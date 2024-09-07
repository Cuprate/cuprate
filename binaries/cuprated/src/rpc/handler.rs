//! Dummy implementation of [`RpcHandler`].

use std::task::{Context, Poll};

use anyhow::Error;
use futures::{channel::oneshot::channel, future::BoxFuture};
use serde::{Deserialize, Serialize};
use tower::Service;

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_helper::asynch::InfallibleOneshotReceiver;
use cuprate_json_rpc::Id;
use cuprate_rpc_interface::RpcHandler;
use cuprate_rpc_types::{
    bin::{BinRequest, BinResponse},
    json::{JsonRpcRequest, JsonRpcResponse},
    other::{OtherRequest, OtherResponse},
};
use cuprate_txpool::service::TxpoolReadHandle;

use crate::rpc::{bin, json, other};

/// TODO
#[derive(Clone)]
pub struct CupratedRpcHandler {
    /// Should this RPC server be [restricted](RpcHandler::restricted)?
    //
    // INVARIANT:
    // We don't need to include this in `state` and check for
    // `self.is_restricted()` because `cuprate-rpc-interface` handles that.
    pub restricted: bool,

    /// State needed for request -> response mapping.
    pub state: CupratedRpcHandlerState,
}

/// TODO
#[derive(Clone)]
pub struct CupratedRpcHandlerState {
    /// Read handle to the blockchain database.
    pub blockchain: BlockchainReadHandle,

    /// Read handle to the transaction pool database.
    pub txpool: TxpoolReadHandle,
}

impl CupratedRpcHandler {
    /// TODO
    pub fn init() {
        todo!()
    }
}

impl RpcHandler for CupratedRpcHandler {
    fn restricted(&self) -> bool {
        self.restricted
    }
}

impl Service<JsonRpcRequest> for CupratedRpcHandler {
    type Response = JsonRpcResponse;
    type Error = Error;
    type Future = BoxFuture<'static, Result<JsonRpcResponse, Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: JsonRpcRequest) -> Self::Future {
        let state = CupratedRpcHandlerState::clone(&self.state);
        Box::pin(json::map_request(state, request))
    }
}

impl Service<BinRequest> for CupratedRpcHandler {
    type Response = BinResponse;
    type Error = Error;
    type Future = BoxFuture<'static, Result<BinResponse, Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: BinRequest) -> Self::Future {
        let state = CupratedRpcHandlerState::clone(&self.state);
        Box::pin(bin::map_request(state, request))
    }
}

impl Service<OtherRequest> for CupratedRpcHandler {
    type Response = OtherResponse;
    type Error = Error;
    type Future = BoxFuture<'static, Result<OtherResponse, Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: OtherRequest) -> Self::Future {
        let state = CupratedRpcHandlerState::clone(&self.state);
        Box::pin(other::map_request(state, request))
    }
}

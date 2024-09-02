//! Dummy implementation of [`RpcHandler`].

//---------------------------------------------------------------------------------------------------- Use
use std::task::Poll;

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

impl Service<RpcRequest> for CupratedRpcHandler {
    type Response = RpcResponse;
    type Error = RpcError;
    type Future = InfallibleOneshotReceiver<Result<RpcResponse, RpcError>>;

    fn poll_ready(&mut self, _: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    /// INVARIANT:
    ///
    /// We don't need to check for `self.is_restricted()`
    /// here because `cuprate-rpc-interface` handles that.
    ///
    /// We can assume the request coming has the required permissions.
    fn call(&mut self, req: RpcRequest) -> Self::Future {
        let state = Self::clone(self);

        let resp = match req {
            RpcRequest::JsonRpc(r) => {
                RpcResponse::JsonRpc(json::map_request(state, r).expect("TODO"))
            } // JSON-RPC 2.0 requests.
            RpcRequest::Binary(r) => RpcResponse::Binary(bin::map_request(state, r).expect("TODO")), // Binary requests.
            RpcRequest::Other(r) => RpcResponse::Other(other::map_request(state, r).expect("TODO")), // JSON (but not JSON-RPC) requests.
        };

        todo!()
        // let (tx, rx) = channel();
        // drop(tx.send(Ok(resp)));
        // InfallibleOneshotReceiver::from(rx)
    }
}

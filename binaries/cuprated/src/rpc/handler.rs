//! Dummy implementation of [`RpcHandler`].

//---------------------------------------------------------------------------------------------------- Use
use std::task::Poll;

use futures::channel::oneshot::channel;
use serde::{Deserialize, Serialize};
use tower::Service;

use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_helper::asynch::InfallibleOneshotReceiver;
use cuprate_json_rpc::Id;
use cuprate_rpc_interface::{RpcError, RpcHandler, RpcRequest, RpcResponse};
use cuprate_txpool::service::{TxpoolReadHandle, TxpoolWriteHandle};

use crate::rpc::{bin, json, other};

//---------------------------------------------------------------------------------------------------- CupratedRpcHandler
/// TODO
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct CupratedRpcHandler {
    /// Should this RPC server be [restricted](RpcHandler::restricted)?
    pub restricted: bool,

    /// Read handle to the blockchain database.
    pub blockchain_read: BlockchainReadHandle,
    /// Write handle to the blockchain database.
    pub blockchain_write: BlockchainWriteHandle,
    /// Direct handle to the blockchain database.
    pub blockchain_db: Arc<ConcreteEnv>,

    /// Read handle to the transaction pool database.
    pub txpool_read: TxpoolReadHandle,
    /// Write handle to the transaction pool database.
    pub txpool_write: TxpoolWriteHandle,
    /// Direct handle to the transaction pool database.
    pub txpool_db: Arc<ConcreteEnv>,
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
        let state = CupratedRpcHandler::clone(self);

        let resp = match req {
            RpcRequest::JsonRpc(r) => json::map_request(state, r), // JSON-RPC 2.0 requests.
            RpcRequest::Binary(r) => bin::map_request(state, r),   // Binary requests.
            RpcRequest::Other(o) => other::map_request(state, r), // JSON (but not JSON-RPC) requests.
        };

        let (tx, rx) = channel();
        drop(tx.send(Ok(resp)));
        InfallibleOneshotReceiver::from(rx)
    }
}

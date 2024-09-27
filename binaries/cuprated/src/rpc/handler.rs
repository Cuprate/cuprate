//! Dummy implementation of [`RpcHandler`].

use std::task::{Context, Poll};

use anyhow::Error;
use futures::future::BoxFuture;
use monero_serai::block::Block;
use tower::Service;

use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_rpc_interface::RpcHandler;
use cuprate_rpc_types::{
    bin::{BinRequest, BinResponse},
    json::{JsonRpcRequest, JsonRpcResponse},
    other::{OtherRequest, OtherResponse},
};
use cuprate_txpool::service::{TxpoolReadHandle, TxpoolWriteHandle};

use crate::rpc::{bin, json, other};

/// TODO: use real type when public.
#[derive(Clone)]
#[expect(clippy::large_enum_variant)]
pub enum BlockchainManagerRequest {
    /// Input is the amount of blocks to pop.
    PopBlocks { amount: usize },

    /// TODO
    Prune,

    /// TODO
    Pruned,

    /// TODO
    RelayBlock(Block),

    /// TODO
    Syncing,

    /// TODO
    Synced,

    /// TODO
    Target,

    /// TODO
    TargetHeight,
}

/// TODO: use real type when public.
#[derive(Clone)]
pub enum BlockchainManagerResponse {
    /// TODO
    ///
    /// Response to:
    /// - [`BlockchainManagerRequest::Prune`]
    /// - [`BlockchainManagerRequest::RelayBlock`]
    Ok,

    /// Response to [`BlockchainManagerRequest::PopBlocks`]
    ///
    /// TODO
    PopBlocks { new_height: usize },

    /// Response to [`BlockchainManagerRequest::Pruned`]
    ///
    /// TODO
    Pruned(bool),

    /// Response to [`BlockchainManagerRequest::Syncing`]
    ///
    /// TODO
    Syncing(bool),

    /// Response to [`BlockchainManagerRequest::Synced`]
    ///
    /// TODO
    Synced(bool),

    /// Response to [`BlockchainManagerRequest::Target`]
    ///
    /// TODO
    Target { height: usize },

    /// Response to [`BlockchainManagerRequest::TargetHeight`]
    ///
    /// TODO
    TargetHeight { height: usize },
}

/// TODO: use real type when public.
pub type BlockchainManagerHandle = cuprate_database_service::DatabaseReadService<
    BlockchainManagerRequest,
    BlockchainManagerResponse,
>;

/// TODO
#[derive(Clone)]
pub struct CupratedRpcHandler {
    /// State needed for request -> response mapping.
    pub state: CupratedRpcHandlerState,
}

/// TODO
#[derive(Clone)]
pub struct CupratedRpcHandlerState {
    /// Should this RPC server be [restricted](RpcHandler::restricted)?
    //
    // INVARIANT:
    // We don't need to include this in `state` and check for
    // `self.is_restricted()` because `cuprate-rpc-interface` handles that.
    pub restricted: bool,

    /// Read handle to the blockchain database.
    pub blockchain_read: BlockchainReadHandle,

    /// Write handle to the blockchain database.
    pub blockchain_write: BlockchainWriteHandle,

    /// Handle to the blockchain manager.
    pub blockchain_manager: BlockchainManagerHandle,

    /// Read handle to the transaction pool database.
    pub txpool_read: TxpoolReadHandle,

    /// Write handle to the transaction pool database.
    pub txpool_write: TxpoolWriteHandle,
}

impl CupratedRpcHandler {
    /// TODO
    pub fn init() {
        todo!()
    }
}

impl RpcHandler for CupratedRpcHandler {
    fn restricted(&self) -> bool {
        self.state.restricted
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

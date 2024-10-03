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
    /// Pop blocks off the top of the blockchain.
    ///
    /// Input is the amount of blocks to pop.
    PopBlocks { amount: usize },

    /// Start pruning the blockchain.
    Prune,

    /// Is the blockchain pruned?
    Pruned,

    /// Relay a block to the network.
    RelayBlock(Block),

    /// Is the blockchain in the middle of syncing?
    ///
    /// This returning `false` does not necessarily
    /// mean [`BlockchainManagerRequest::Synced`] will
    /// return `true`, for example, if the network has been
    /// cut off and we have no peers, this will return `false`,
    /// however, [`BlockchainManagerRequest::Synced`] may return
    /// `true` if the latest known chain tip is equal to our height.
    Syncing,

    /// Is the blockchain fully synced?
    Synced,

    /// Current target block time.
    Target,

    /// The height of the next block in the chain.
    TargetHeight,
}

/// TODO: use real type when public.
#[derive(Clone)]
pub enum BlockchainManagerResponse {
    /// General OK response.
    ///
    /// Response to:
    /// - [`BlockchainManagerRequest::Prune`]
    /// - [`BlockchainManagerRequest::RelayBlock`]
    Ok,

    /// Response to [`BlockchainManagerRequest::PopBlocks`]
    PopBlocks { new_height: usize },

    /// Response to [`BlockchainManagerRequest::Pruned`]
    Pruned(bool),

    /// Response to [`BlockchainManagerRequest::Syncing`]
    Syncing(bool),

    /// Response to [`BlockchainManagerRequest::Synced`]
    Synced(bool),

    /// Response to [`BlockchainManagerRequest::Target`]
    Target(std::time::Duration),

    /// Response to [`BlockchainManagerRequest::TargetHeight`]
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
    /// Should this RPC server be [restricted](RpcHandler::restricted)?
    ///
    /// This is not `pub` on purpose, as it should not be mutated after [`Self::new`].
    restricted: bool,

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
    /// Create a new [`Self`].
    pub const fn new(
        restricted: bool,
        blockchain_read: BlockchainReadHandle,
        blockchain_write: BlockchainWriteHandle,
        blockchain_manager: BlockchainManagerHandle,
        txpool_read: TxpoolReadHandle,
        txpool_write: TxpoolWriteHandle,
    ) -> Self {
        Self {
            restricted,
            blockchain_read,
            blockchain_write,
            blockchain_manager,
            txpool_read,
            txpool_write,
        }
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
        let state = self.clone();
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
        let state = self.clone();
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
        let state = self.clone();
        Box::pin(other::map_request(state, request))
    }
}

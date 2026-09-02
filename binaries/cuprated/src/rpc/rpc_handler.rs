//! `cuprated`'s implementation of [`RpcHandler`].

use std::task::{Context, Poll};

use anyhow::Error;
use futures::future::BoxFuture;
use tower::Service;

use cuprate_blockchain::service::BlockchainReadHandle;
use cuprate_consensus::BlockchainContextService;
use cuprate_helper::network::Network;
use cuprate_rpc_interface::RpcHandler;

use cuprate_rpc_types::{
    bin::{BinRequest, BinResponse},
    json::{JsonRpcRequest, JsonRpcResponse},
    other::{OtherRequest, OtherResponse},
};
use cuprate_txpool::service::TxpoolReadHandle;

use crate::{
    blockchain::{BlockchainManagerHandle, BlockchainSyncerHandle},
    monitor::TaskExecutor,
    rpc::handlers,
    txpool::IncomingTxHandler,
    LaunchContext,
};

/// cuprated's RPC handler service.
#[derive(Clone)]
pub(crate) struct CupratedRpcHandler {
    /// Should this RPC server be [restricted](RpcHandler::is_restricted)?
    ///
    /// This is not `pub` on purpose, as it should not be mutated after [`Self::new`].
    restricted: bool,

    /// The active network.
    pub network: Network,

    /// Is the node running offline?
    pub offline: bool,

    /// Read handle to the blockchain database.
    pub blockchain_read: BlockchainReadHandle,

    /// Handle to the blockchain context service.
    pub blockchain_context: BlockchainContextService,

    /// Read handle to the transaction pool database.
    pub txpool_read: TxpoolReadHandle,

    pub tx_handler: IncomingTxHandler,

    /// Handle to the blockchain syncer.
    pub blockchain_syncer: BlockchainSyncerHandle,

    /// Command channel to the blockchain manager.
    pub blockchain_manager: BlockchainManagerHandle,

    /// The time this node was launched as a UNIX timestamp.
    pub start_instant_unix: u64,

    /// Task spawning and shutdown coordination.
    pub task_executor: TaskExecutor,
}

impl CupratedRpcHandler {
    /// Create a new [`Self`].
    pub(crate) fn new(
        restricted: bool,
        tx_handler: IncomingTxHandler,
        launch_ctx: &LaunchContext,
    ) -> Self {
        let start_instant_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            restricted,
            network: launch_ctx.config.network,
            offline: launch_ctx.config.offline,
            tx_handler,
            blockchain_read: launch_ctx.blockchain.read(),
            blockchain_context: launch_ctx.blockchain.context_svc(),
            txpool_read: launch_ctx.txpool_read.clone(),
            blockchain_syncer: launch_ctx.blockchain.syncer(),
            blockchain_manager: launch_ctx.blockchain.manager(),
            start_instant_unix,
            task_executor: launch_ctx.task_executor.clone(),
        }
    }
}

impl RpcHandler for CupratedRpcHandler {
    fn is_restricted(&self) -> bool {
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
        Box::pin(handlers::json_rpc::map_request(state, request))
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
        Box::pin(handlers::bin::map_request(state, request))
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
        Box::pin(handlers::other_json::map_request(state, request))
    }
}

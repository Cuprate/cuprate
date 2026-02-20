//! `cuprated`'s implementation of [`RpcHandler`].

use std::task::{Context, Poll};

use anyhow::Error;
use futures::future::BoxFuture;
use monero_oxide::block::Block;
use tokio_util::sync::CancellationToken;
use tower::Service;

use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::BlockchainContextService;
use cuprate_pruning::PruningSeed;
use cuprate_rpc_interface::RpcHandler;
use cuprate_rpc_types::{
    bin::{BinRequest, BinResponse},
    json::{JsonRpcRequest, JsonRpcResponse},
    other::{OtherRequest, OtherResponse},
};
use cuprate_txpool::service::TxpoolReadHandle;
use cuprate_types::BlockTemplate;

use crate::{rpc::handlers, txpool::IncomingTxHandler};

/// TODO: use real type when public.
#[derive(Clone)]
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
    RelayBlock(
        /// This is [`Box`]ed due to `clippy::large_enum_variant`.
        Box<Block>,
    ),

    /// Sync/flush the blockchain database to disk.
    Sync,

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

    /// Generate new blocks.
    ///
    /// This request is only for regtest, see RPC's `generateblocks`.
    GenerateBlocks {
        /// Number of the blocks to be generated.
        amount_of_blocks: u64,
        /// The previous block's hash.
        prev_block: Option<[u8; 32]>,
        /// The starting value for the nonce.
        starting_nonce: u32,
        /// The address that will receive the coinbase reward.
        wallet_address: String,
    },

    //    // TODO: the below requests actually belong to the block downloader/syncer:
    //    // <https://github.com/Cuprate/cuprate/pull/320#discussion_r1811089758>
    //    /// Get [`Span`] data.
    //    ///
    //    /// This is data that describes an active downloading process,
    //    /// if we are fully synced, this will return an empty [`Vec`].
    //    Spans,

    //
    /// Get the next [`PruningSeed`] needed for a pruned sync.
    NextNeededPruningSeed,

    /// Create a block template.
    CreateBlockTemplate {
        prev_block: [u8; 32],
        account_public_address: String,
        extra_nonce: Vec<u8>,
    },

    /// Safely shutdown `cuprated`.
    Stop,
}

/// TODO: use real type when public.
#[derive(Clone)]
pub enum BlockchainManagerResponse {
    /// General OK response.
    ///
    /// Response to:
    /// - [`BlockchainManagerRequest::Prune`]
    /// - [`BlockchainManagerRequest::RelayBlock`]
    /// - [`BlockchainManagerRequest::Sync`]
    Ok,

    /// Response to [`BlockchainManagerRequest::PopBlocks`]
    PopBlocks { new_height: usize },

    /// Response to [`BlockchainManagerRequest::Prune`]
    Prune(PruningSeed),

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

    /// Response to [`BlockchainManagerRequest::GenerateBlocks`]
    GenerateBlocks {
        /// Hashes of the blocks generated.
        blocks: Vec<[u8; 32]>,
        /// The new top height. (TODO: is this correct?)
        height: usize,
    },

    /// Response to [`BlockchainManagerRequest::NextNeededPruningSeed`].
    NextNeededPruningSeed(PruningSeed),

    /// Response to [`BlockchainManagerRequest::CreateBlockTemplate`].
    CreateBlockTemplate(Box<BlockTemplate>),
}

/// TODO: use real type when public.
pub type BlockchainManagerHandle = cuprate_database_service::DatabaseReadService<
    BlockchainManagerRequest,
    BlockchainManagerResponse,
>;

/// cuprated's RPC handler service.
#[derive(Clone)]
pub struct CupratedRpcHandler {
    /// Should this RPC server be [restricted](RpcHandler::is_restricted)?
    ///
    /// This is not `pub` on purpose, as it should not be mutated after [`Self::new`].
    restricted: bool,

    /// Read handle to the blockchain database.
    pub blockchain_read: BlockchainReadHandle,

    /// Handle to the blockchain context service.
    pub blockchain_context: BlockchainContextService,

    /// Read handle to the transaction pool database.
    pub txpool_read: TxpoolReadHandle,

    pub tx_handler: IncomingTxHandler,

    /// Cancellation token used to trigger a graceful shutdown.
    pub shutdown_token: CancellationToken,
}

impl CupratedRpcHandler {
    /// Create a new [`Self`].
    pub const fn new(
        restricted: bool,
        blockchain_read: BlockchainReadHandle,
        blockchain_context: BlockchainContextService,
        txpool_read: TxpoolReadHandle,
        tx_handler: IncomingTxHandler,
        shutdown_token: CancellationToken,
    ) -> Self {
        Self {
            restricted,
            blockchain_read,
            blockchain_context,
            txpool_read,
            tx_handler,
            shutdown_token,
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

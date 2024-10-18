//! Dummy implementation of [`RpcHandler`].

use std::task::{Context, Poll};

use anyhow::Error;
use futures::future::BoxFuture;
use monero_serai::block::Block;
use tower::Service;

use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::BlockChainContextService;
use cuprate_pruning::PruningSeed;
use cuprate_rpc_interface::RpcHandler;
use cuprate_rpc_types::{
    bin::{BinRequest, BinResponse},
    json::{JsonRpcRequest, JsonRpcResponse},
    other::{OtherRequest, OtherResponse},
};
use cuprate_txpool::service::{TxpoolReadHandle, TxpoolWriteHandle};
use cuprate_types::{AddAuxPow, AuxPow, HardFork};

use crate::rpc::{bin, json, other};

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

    /// Calculate proof-of-work for this block.
    CalculatePow {
        /// The hardfork of the protocol at this block height.
        hardfork: HardFork,
        /// The height of the block.
        height: usize,
        /// The block data.
        block: Block,
        /// The seed hash for the proof-of-work.
        seed_hash: [u8; 32],
    },

    /// Add auxirilly proof-of-work to a block.
    ///
    /// From the RPC `add_aux_pow` usecase's documentation:
    /// ````
    /// This enables merge mining with Monero without requiring
    /// software that manually alters the extra field in the coinbase
    /// tx to include the merkle root of the aux blocks.
    /// ````
    AddAuxPow {
        /// The block template to add to.
        block_template: Block,
        /// The auxirilly proof-of-work to add.
        aux_pow: Vec<AuxPow>,
    },

    /// Generate new blocks.
    ///
    /// This request is only for regtest, see RPC's `generateblocks`.
    GenerateBlocks {
        /// Number of the blocks to be generated.
        amount_of_blocks: u64,
        /// The previous block's hash.
        prev_block: [u8; 32],
        /// The starting value for the nonce.
        starting_nonce: u32,
        /// The address that will receive the coinbase reward.
        wallet_address: String,
    },

    /// Get a visual [`String`] overview of blockchain progress.
    ///
    /// This is a highly implementation specific format used by
    /// `monerod` in the `sync_info` RPC call's `overview` field;
    /// it is essentially an ASCII visual of blocks.
    ///
    /// See also:
    /// - <https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#sync_info>
    /// - <https://github.com/monero-project/monero/blob/master/src/cryptonote_protocol/block_queue.cpp#L178>
    Overview {
        /// TODO: the current blockchain height? do we need to pass this?
        height: usize,
    },
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

    /// Response to [`BlockchainManagerRequest::CalculatePow`]
    CalculatePow([u8; 32]),

    /// Response to [`BlockchainManagerRequest::AddAuxPow`]
    AddAuxPow(AddAuxPow),

    /// Response to [`BlockchainManagerRequest::GenerateBlocks`]
    GenerateBlocks {
        /// Hashes of the blocks generated.
        blocks: Vec<[u8; 32]>,
        /// The new top height. (TODO: is this correct?)
        height: usize,
    },

    /// Response to [`BlockchainManagerRequest::Overview`]
    Overview(String),
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

    /// Handle to the blockchain context service.
    pub blockchain_context: BlockChainContextService,

    /// Handle to the blockchain manager.
    pub blockchain_manager: BlockchainManagerHandle,

    /// Read handle to the transaction pool database.
    pub txpool_read: TxpoolReadHandle,

    /// TODO: handle to txpool service.
    pub txpool_manager: std::convert::Infallible,
}

impl CupratedRpcHandler {
    /// Create a new [`Self`].
    pub const fn new(
        restricted: bool,
        blockchain_read: BlockchainReadHandle,
        blockchain_context: BlockChainContextService,
        blockchain_manager: BlockchainManagerHandle,
        txpool_read: TxpoolReadHandle,
        txpool_manager: std::convert::Infallible,
    ) -> Self {
        Self {
            restricted,
            blockchain_read,
            blockchain_context,
            blockchain_manager,
            txpool_read,
            txpool_manager,
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

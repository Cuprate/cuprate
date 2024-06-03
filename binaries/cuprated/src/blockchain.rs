//! # The Syncer
//!
//! The syncer is the part of Cuprate that handles keeping the blockchain state, it handles syncing if
//! we have fallen behind, and it handles incoming blocks.
use cuprate_blockchain::service::DatabaseWriteHandle;
use monero_serai::{block::Block, transaction::Transaction};
use tokio::sync::mpsc;
use tower::Service;

use cuprate_consensus::{
    BlockChainContextRequest, BlockChainContextResponse, ExtendedConsensusError,
    VerifyBlockRequest, VerifyBlockResponse,
};
use monero_p2p::handles::ConnectionHandle;

pub struct IncomingBlock {
    block: Block,
    included_txs: Vec<Transaction>,
    peer_handle: ConnectionHandle,
}

/// A response to an [`IncomingBlock`]
pub enum IncomingBlockResponse {
    /// We are missing these transactions from the block.
    MissingTransactions(Vec<[u8; 32]>),
    /// A generic ok response.
    Ok,
}

struct BlockBatch;

/// The blockchain.
///
/// This struct represents the task that syncs and maintains Cuprate's blockchain state.
pub struct Blockchain<C, BV> {
    /// The blockchain context service.
    ///
    /// This service handles keeping all the data needed to verify new blocks.
    context_svc: C,
    /// The block verifier service, handles block verification.
    block_verifier_svc: BV,

    /// The blockchain database write handle.
    database_svc: DatabaseWriteHandle,

    incoming_block_rx: mpsc::Receiver<IncomingBlock>,

    incoming_block_batch_rx: mpsc::Receiver<BlockBatch>,
}

impl<C, BV> Blockchain<C, BV>
where
    C: Service<
        BlockChainContextRequest,
        Response = BlockChainContextResponse,
        Error = tower::BoxError,
    >,
    C::Future: Send + 'static,
    BV: Service<VerifyBlockRequest, Response = VerifyBlockResponse, Error = ExtendedConsensusError>,
    BV::Future: Send + 'static,
{
}

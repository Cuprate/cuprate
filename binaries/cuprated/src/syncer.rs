//! # The Syncer
//!
//! The syncer is the part of Cuprate that handles keeping the blockchain state, it handles syncing if
//! we have fallen behind, and it handles incoming blocks.
use monero_serai::{block::Block, transaction::Transaction};

use monero_p2p::handles::ConnectionHandle;

pub struct IncomingFluffyBlock {
    block: Block,
    included_txs: Vec<Transaction>,
    peer_handle: ConnectionHandle,
}

/// A response to an [`IncomingFluffyBlock`]
pub enum IncomingFluffyBlockResponse {
    /// We are missing these transactions from the block.
    MissingTransactions(Vec<[u8; 32]>),
    /// A generic ok response.
    Ok,
}

//! Error types for the blockchain manager interface.

use cuprate_blockchain::BlockchainError;
use cuprate_consensus_rules::{blocks::BlockError, hard_forks::HardForkError, ConsensusError};
use cuprate_txpool::TxPoolError;

use crate::monitor::FatalError;

/// A validation failure - the peer should be banned.
#[derive(Debug, thiserror::Error)]
pub enum BlockValidationError {
    /// Invalid hard-fork rules.
    #[error(transparent)]
    HardFork(HardForkError),

    /// Any other consensus rule violation.
    #[error(transparent)]
    Consensus(ConsensusError),
}

impl From<ConsensusError> for BlockValidationError {
    fn from(e: ConsensusError) -> Self {
        match e {
            ConsensusError::Block(BlockError::HardForkError(hf)) => Self::HardFork(hf),
            ConsensusError::Block(e) => Self::Consensus(ConsensusError::Block(e)),
            ConsensusError::Transaction(e) => Self::Consensus(ConsensusError::Transaction(e)),
        }
    }
}

/// An error returned from [`BlockchainManagerHandle::handle_incoming_block`](super::interface::BlockchainManagerHandle::handle_incoming_block).
#[derive(Debug, thiserror::Error)]
pub enum IncomingBlockError {
    /// The peer sent us an invalid block; ban them.
    #[error(transparent)]
    Validation(BlockValidationError),

    /// We cannot recover; shut the node down.
    #[error(transparent)]
    Fatal(FatalError),

    /// We are missing the block's parent.
    #[error("The block has an unknown parent.")]
    Orphan,

    /// Some transactions in the block were unknown.
    ///
    /// The inner values are the block hash and the indexes of the missing txs in the block.
    #[error("Unknown transactions in block.")]
    UnknownTransactions([u8; 32], Vec<usize>),

    /// The block claimed more transactions than it contained.
    #[error("Too many transactions given for block.")]
    TooManyTxs,

    /// The blockchain manager command channel is closed.
    #[error("The blockchain manager command channel is closed.")]
    ChannelClosed,
}

impl From<ConsensusError> for IncomingBlockError {
    fn from(e: ConsensusError) -> Self {
        Self::Validation(e.into())
    }
}

impl From<BlockchainError> for IncomingBlockError {
    fn from(e: BlockchainError) -> Self {
        Self::Fatal(e.into())
    }
}

impl From<TxPoolError> for IncomingBlockError {
    fn from(e: TxPoolError) -> Self {
        Self::Fatal(e.into())
    }
}

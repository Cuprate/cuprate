//! Error types for the blockchain manager interface.

use cuprate_blockchain::BlockchainError;
use cuprate_consensus::{block::BlockVerificationError, ExtendedConsensusError};
use cuprate_consensus_rules::ConsensusError;
use cuprate_txpool::TxPoolError;

use crate::monitor::FatalError;

/// An error returned from [`BlockchainManagerHandle::handle_incoming_block`](super::interface::BlockchainManagerHandle::handle_incoming_block).
#[derive(Debug, thiserror::Error)]
pub enum IncomingBlockError {
    /// The peer sent us an invalid block.
    #[error("Block verification failed: {inner}")]
    Validation {
        /// Whether the block's proof-of-work was verified before the failure.
        pow_valid: bool,
        /// The consensus rule that was broken.
        inner: ConsensusError,
    },

    /// We cannot recover; shut the node down.
    #[error(transparent)]
    Fatal(#[from] FatalError),

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

impl From<BlockVerificationError> for IncomingBlockError {
    fn from(error: BlockVerificationError) -> Self {
        let BlockVerificationError { pow_valid, inner } = error;

        match inner {
            ExtendedConsensusError::FatalError(error) => Self::Fatal(error),
            ExtendedConsensusError::ConsensusError(inner) => Self::Validation { pow_valid, inner },
        }
    }
}

impl From<BlockchainError> for IncomingBlockError {
    fn from(e: BlockchainError) -> Self {
        Self::Fatal(e.into())
    }
}

impl From<TxPoolError> for IncomingBlockError {
    fn from(v: TxPoolError) -> Self {
        Self::Fatal(v.into())
    }
}

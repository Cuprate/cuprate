//! Error types for the blockchain manager interface.

use cuprate_blockchain::BlockchainError;
use cuprate_consensus::ExtendedConsensusError;
use cuprate_consensus_rules::ConsensusError;
use cuprate_txpool::TxPoolError;
use cuprate_types::TxConversionError;

use crate::txpool::TxPoolManagerClosed;

/// An error that can be returned from [`BlockchainManagerHandle::handle_incoming_block`](super::interface::BlockchainManagerHandle::handle_incoming_block).
#[derive(Debug, thiserror::Error)]
pub enum IncomingBlockError {
    /// Some transactions in the block were unknown.
    ///
    /// The inner values are the block hash and the indexes of the missing txs in the block.
    #[error("Unknown transactions in block.")]
    UnknownTransactions([u8; 32], Vec<usize>),

    /// We are missing the block's parent.
    #[error("The block has an unknown parent.")]
    Orphan,

    /// The block was received as an alt block but already exists on the
    /// main chain.
    #[error("Alt block already in main chain.")]
    AlreadyInMainChain,

    /// The block claimed more transactions than it contained.
    #[error("Too many transactions given for block.")]
    TooManyTxs,

    /// The block failed consensus validation.
    #[error(transparent)]
    Consensus(ExtendedConsensusError),

    /// An inner tower service returned an error.
    #[error(transparent)]
    Service(#[from] tower::BoxError),

    /// The blockchain manager command channel is closed.
    #[error("The blockchain manager command channel is closed.")]
    ChannelClosed,
}

impl From<ExtendedConsensusError> for IncomingBlockError {
    fn from(e: ExtendedConsensusError) -> Self {
        if let ExtendedConsensusError::DBErr(e) = e {
            return Self::Service(e);
        }
        Self::Consensus(e)
    }
}

impl From<ConsensusError> for IncomingBlockError {
    fn from(e: ConsensusError) -> Self {
        Self::Consensus(e.into())
    }
}

impl From<BlockchainError> for IncomingBlockError {
    fn from(e: BlockchainError) -> Self {
        Self::Service(e.into())
    }
}

impl From<TxPoolError> for IncomingBlockError {
    fn from(e: TxPoolError) -> Self {
        Self::Service(e.into())
    }
}

impl From<TxConversionError> for IncomingBlockError {
    fn from(e: TxConversionError) -> Self {
        Self::Service(e.into())
    }
}

impl From<TxPoolManagerClosed> for IncomingBlockError {
    fn from(e: TxPoolManagerClosed) -> Self {
        Self::Service(e.into())
    }
}

impl IncomingBlockError {
    /// Returns `true` if the error suggests the peer that supplied the
    /// block behaved incorrectly.
    pub const fn is_peer_fault(&self) -> bool {
        matches!(
            self,
            Self::UnknownTransactions(..)
                | Self::AlreadyInMainChain
                | Self::TooManyTxs
                | Self::Consensus(_)
        )
    }
}

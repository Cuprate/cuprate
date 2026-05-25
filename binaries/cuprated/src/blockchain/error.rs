//! Error types for the blockchain manager interface.

use cuprate_blockchain::BlockchainError;
use cuprate_consensus::ExtendedConsensusError;
use cuprate_consensus_rules::ConsensusError;
use cuprate_txpool::TxPoolError;
use cuprate_types::TxConversionError;

macro_rules! impl_internal_from {
    ($($t:ty),* $(,)?) => {$(
        impl From<$t> for BlockManagerError {
            fn from(e: $t) -> Self { Self::Internal(e.into()) }
        }
        impl From<$t> for IncomingBlockError {
            fn from(e: $t) -> Self { BlockManagerError::from(e).into() }
        }
    )*};
}

/// A validation failure - the peer should be banned.
#[derive(Debug, thiserror::Error)]
pub enum BlockValidationError {
    /// The block was received as an alt block but already exists on the
    /// main chain.
    #[error("Alt block already in main chain.")]
    AlreadyInMainChain,

    /// The block failed consensus validation.
    #[error(transparent)]
    Consensus(ExtendedConsensusError),
}

/// An error from the blockchain manager's internal handlers.
#[derive(Debug, thiserror::Error)]
pub enum BlockManagerError {
    /// The peer sent us an invalid block; ban them.
    #[error(transparent)]
    Validation(#[from] BlockValidationError),

    /// A node-side failure.
    #[error(transparent)]
    Internal(#[from] tower::BoxError),
}

impl From<ExtendedConsensusError> for BlockManagerError {
    fn from(e: ExtendedConsensusError) -> Self {
        if let ExtendedConsensusError::DBErr(e) = e {
            return Self::Internal(e);
        }
        BlockValidationError::Consensus(e).into()
    }
}

impl From<ConsensusError> for BlockManagerError {
    fn from(e: ConsensusError) -> Self {
        BlockValidationError::Consensus(e.into()).into()
    }
}

/// An error returned from [`BlockchainManagerHandle::handle_incoming_block`](super::interface::BlockchainManagerHandle::handle_incoming_block).
#[derive(Debug, thiserror::Error)]
pub enum IncomingBlockError {
    /// The peer sent us an invalid block; ban them.
    #[error(transparent)]
    Validation(BlockValidationError),

    /// A node-side failure.
    #[error(transparent)]
    Internal(tower::BoxError),

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

impl From<BlockManagerError> for IncomingBlockError {
    fn from(e: BlockManagerError) -> Self {
        match e {
            BlockManagerError::Validation(v) => Self::Validation(v),
            BlockManagerError::Internal(i) => Self::Internal(i),
        }
    }
}

impl From<ConsensusError> for IncomingBlockError {
    fn from(e: ConsensusError) -> Self {
        BlockManagerError::from(e).into()
    }
}

impl_internal_from!(BlockchainError, TxPoolError, TxConversionError);

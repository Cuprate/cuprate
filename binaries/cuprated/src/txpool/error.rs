//! Error types for incoming transaction handling.

use cuprate_consensus::ExtendedConsensusError;
use cuprate_consensus_rules::ConsensusError;
use cuprate_dandelion_tower::pool::DandelionPoolShutDown;
use cuprate_txpool::TxPoolError;

use crate::{monitor::FatalError, txpool::relay_rules::RelayRuleError};

/// An error returned while handling an incoming transaction.
#[derive(Debug, thiserror::Error)]
pub(crate) enum IncomingTxError {
    /// The transaction could not be parsed.
    #[error("Error parsing tx: {0}")]
    Parse(#[from] std::io::Error),

    /// The transaction violated a consensus rule.
    #[error(transparent)]
    Consensus(ConsensusError),

    /// A duplicate transaction appeared in the incoming batch.
    #[error("Duplicate tx in message.")]
    DuplicateTransaction,

    /// A relay rule was broken.
    #[error("Relay rule was broken: {0}")]
    RelayRule(#[from] RelayRuleError),

    /// We cannot recover; shut the node down.
    #[error(transparent)]
    Fatal(#[from] FatalError),

    /// The tx-pool manager command channel is closed.
    #[error("The tx-pool manager command channel is closed.")]
    ChannelClosed,
}

impl From<ExtendedConsensusError> for IncomingTxError {
    fn from(e: ExtendedConsensusError) -> Self {
        match e {
            ExtendedConsensusError::FatalError(e) => Self::Fatal(e),
            ExtendedConsensusError::ConsensusError(e) => Self::Consensus(e),
        }
    }
}

impl From<ConsensusError> for IncomingTxError {
    fn from(e: ConsensusError) -> Self {
        Self::Consensus(e)
    }
}

impl From<TxPoolError> for IncomingTxError {
    fn from(e: TxPoolError) -> Self {
        Self::Fatal(e.into())
    }
}

impl From<DandelionPoolShutDown> for IncomingTxError {
    fn from(e: DandelionPoolShutDown) -> Self {
        Self::Fatal(e.into())
    }
}

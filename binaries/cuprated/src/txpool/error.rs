//! Error types for incoming transaction handling.

use cuprate_consensus::ExtendedConsensusError;
use cuprate_txpool::TxPoolError;

use crate::txpool::relay_rules::RelayRuleError;

/// An error returned while handling an incoming transaction.
///
/// Callers use [`IncomingTxError::is_peer_fault`] to decide whether to
/// punish the peer that supplied the transaction.
#[derive(Debug, thiserror::Error)]
pub enum IncomingTxError {
    /// The transaction could not be parsed.
    #[error("Error parsing tx: {0}")]
    Parse(#[from] std::io::Error),

    /// The transaction violated a consensus rule.
    #[error(transparent)]
    Consensus(ExtendedConsensusError),

    /// A duplicate transaction appeared in the incoming batch.
    #[error("Duplicate tx in message.")]
    DuplicateTransaction,

    /// A relay rule was broken.
    #[error("Relay rule was broken: {0}")]
    RelayRule(#[from] RelayRuleError),

    /// An inner tower service returned an error.
    #[error(transparent)]
    Service(#[from] tower::BoxError),
}

impl From<ExtendedConsensusError> for IncomingTxError {
    fn from(e: ExtendedConsensusError) -> Self {
        if let ExtendedConsensusError::DBErr(e) = e {
            return Self::Service(e);
        }
        Self::Consensus(e)
    }
}

impl From<TxPoolError> for IncomingTxError {
    fn from(e: TxPoolError) -> Self {
        Self::Service(e.into())
    }
}

impl IncomingTxError {
    /// Returns `true` if the error suggests the peer that supplied the
    /// transaction behaved incorrectly.
    pub const fn is_peer_fault(&self) -> bool {
        matches!(
            self,
            Self::Parse(_) | Self::Consensus(_) | Self::DuplicateTransaction | Self::RelayRule(_)
        )
    }
}

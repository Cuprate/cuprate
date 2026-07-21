//! Error types for incoming transaction handling.

use cuprate_consensus::ExtendedConsensusError;
use cuprate_consensus_rules::ConsensusError;

use crate::txpool::relay_rules::RelayRuleError;

/// A validation failure - the tx should be dropped.
#[derive(Debug, thiserror::Error)]
pub enum TxValidationError {
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
}

/// An error returned while handling an incoming transaction.
#[derive(Debug, thiserror::Error)]
pub enum IncomingTxError {
    /// The tx was rejected by validation; drop it.
    #[error(transparent)]
    Validation(#[from] TxValidationError),

    /// An inner tower service returned an error.
    #[error(transparent)]
    Internal(#[from] tower::BoxError),
}

impl From<ExtendedConsensusError> for IncomingTxError {
    fn from(e: ExtendedConsensusError) -> Self {
        match e {
            ExtendedConsensusError::DBErr(e) => Self::Internal(e),

            ExtendedConsensusError::ConErr(_)
            | ExtendedConsensusError::TxsIncludedWithBlockIncorrect
            | ExtendedConsensusError::OneOrMoreBatchVerificationStatementsInvalid
            | ExtendedConsensusError::NoBlocksToVerify => TxValidationError::Consensus(e).into(),
        }
    }
}

impl From<ConsensusError> for IncomingTxError {
    fn from(e: ConsensusError) -> Self {
        TxValidationError::Consensus(e.into()).into()
    }
}

impl From<std::io::Error> for IncomingTxError {
    fn from(e: std::io::Error) -> Self {
        TxValidationError::from(e).into()
    }
}

impl From<RelayRuleError> for IncomingTxError {
    fn from(e: RelayRuleError) -> Self {
        TxValidationError::from(e).into()
    }
}

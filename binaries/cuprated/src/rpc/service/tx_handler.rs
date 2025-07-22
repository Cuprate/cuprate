use anyhow::{anyhow, Error};
use cuprate_consensus::ExtendedConsensusError;
use cuprate_consensus_rules::{transactions::TransactionError, ConsensusError};
use tower::{Service, ServiceExt};

use cuprate_types::TxRelayChecks;

use crate::txpool::{IncomingTxError, IncomingTxHandler, IncomingTxs, RelayRuleError};

pub async fn handle_incoming_txs(
    tx_handler: &mut IncomingTxHandler,
    incoming_txs: IncomingTxs,
) -> Result<TxRelayChecks, Error> {
    let resp = tx_handler
        .ready()
        .await
        .map_err(|e| anyhow!(e))?
        .call(incoming_txs)
        .await;

    Ok(match resp {
        Ok(()) => TxRelayChecks::empty(),
        Err(e) => match e {
            IncomingTxError::Consensus(ExtendedConsensusError::ConErr(
                ConsensusError::Transaction(e),
            )) => match e {
                TransactionError::TooBig => TxRelayChecks::TOO_BIG,
                TransactionError::KeyImageSpent => TxRelayChecks::DOUBLE_SPEND,

                TransactionError::OutputNotValidPoint
                | TransactionError::OutputTypeInvalid
                | TransactionError::ZeroOutputForV1
                | TransactionError::NonZeroOutputForV2
                | TransactionError::OutputsOverflow
                | TransactionError::OutputsTooHigh => TxRelayChecks::INVALID_OUTPUT,

                TransactionError::MoreThanOneMixableInputWithUnmixable
                | TransactionError::InvalidNumberOfOutputs
                | TransactionError::InputDoesNotHaveExpectedNumbDecoys
                | TransactionError::IncorrectInputType
                | TransactionError::InputsAreNotOrdered
                | TransactionError::InputsOverflow
                | TransactionError::NoInputs => TxRelayChecks::INVALID_INPUT,

                TransactionError::KeyImageIsNotInPrimeSubGroup
                | TransactionError::AmountNotDecomposed
                | TransactionError::DuplicateRingMember
                | TransactionError::OneOrMoreRingMembersLocked
                | TransactionError::RingMemberNotFoundOrInvalid
                | TransactionError::RingSignatureIncorrect
                | TransactionError::TransactionVersionInvalid
                | TransactionError::RingCTError(_) => return Err(anyhow!("unreachable")),
            },
            IncomingTxError::Parse(_) | IncomingTxError::Consensus(_) => {
                return Err(anyhow!("unreachable"))
            }
            IncomingTxError::RelayRule(RelayRuleError::NonZeroTimelock) => {
                TxRelayChecks::NONZERO_UNLOCK_TIME
            }
            IncomingTxError::RelayRule(RelayRuleError::ExtraFieldTooLarge) => {
                TxRelayChecks::TX_EXTRA_TOO_BIG
            }
            IncomingTxError::RelayRule(RelayRuleError::FeeBelowMinimum) => {
                TxRelayChecks::FEE_TOO_LOW
            }
            IncomingTxError::DuplicateTransaction => TxRelayChecks::DOUBLE_SPEND,
        },
    })
}

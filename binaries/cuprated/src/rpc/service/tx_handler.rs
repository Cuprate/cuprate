use anyhow::{anyhow, Error};
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
            IncomingTxError::Parse(_) | IncomingTxError::Consensus(_) => {
                TxRelayChecks::INVALID_INPUT | TxRelayChecks::INVALID_OUTPUT
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

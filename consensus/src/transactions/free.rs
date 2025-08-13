use monero_serai::{
    ringct::{bulletproofs::Bulletproof, RctType},
    transaction::{Input, Transaction},
};

use cuprate_consensus_rules::{transactions::TransactionError, ConsensusError};
use cuprate_types::{CachedVerificationState, TransactionVerificationData, TxVersion};

/// Creates a new [`TransactionVerificationData`] from a [`Transaction`].
///
/// # Errors
///
/// This function will return [`Err`] if the transaction is malformed, although returning [`Ok`] does
/// not necessarily mean the tx is correctly formed.
pub fn new_tx_verification_data(
    tx: Transaction,
) -> Result<TransactionVerificationData, ConsensusError> {
    let tx_hash = tx.hash();
    let tx_blob = tx.serialize();

    let tx_weight = tx_weight(&tx, &tx_blob);

    let fee = tx_fee(&tx)?;

    Ok(TransactionVerificationData {
        tx_hash,
        version: TxVersion::from_raw(tx.version())
            .ok_or(TransactionError::TransactionVersionInvalid)?,
        tx_blob,
        tx_weight,
        fee,
        cached_verification_state: CachedVerificationState::NotVerified,
        tx,
    })
}

/// Calculates the weight of a [`Transaction`].
///
/// This is more efficient that [`Transaction::weight`] if you already have the transaction blob.
pub(crate) fn tx_weight(tx: &Transaction, tx_blob: &[u8]) -> usize {
    // the tx weight is only different from the blobs length for bp(+) txs.

    match &tx {
        Transaction::V1 { .. } | Transaction::V2 { proofs: None, .. } => tx_blob.len(),
        Transaction::V2 {
            proofs: Some(proofs),
            ..
        } => match proofs.rct_type() {
            RctType::AggregateMlsagBorromean | RctType::MlsagBorromean => tx_blob.len(),
            RctType::MlsagBulletproofs
            | RctType::MlsagBulletproofsCompactAmount
            | RctType::ClsagBulletproof => {
                tx_blob.len()
                    + Bulletproof::calculate_bp_clawback(false, tx.prefix().outputs.len()).0
            }
            RctType::ClsagBulletproofPlus => {
                tx_blob.len()
                    + Bulletproof::calculate_bp_clawback(true, tx.prefix().outputs.len()).0
            }
        },
    }
}

/// Calculates the fee of the [`Transaction`].
pub(crate) fn tx_fee(tx: &Transaction) -> Result<u64, TransactionError> {
    let mut fee = 0_u64;

    match &tx {
        Transaction::V1 { prefix, .. } => {
            for input in &prefix.inputs {
                if let Input::ToKey { amount, .. } = input {
                    fee = fee
                        .checked_add(amount.unwrap_or(0))
                        .ok_or(TransactionError::InputsOverflow)?;
                }
            }

            for output in &prefix.outputs {
                fee = fee
                    .checked_sub(output.amount.unwrap_or(0))
                    .ok_or(TransactionError::OutputsTooHigh)?;
            }
        }
        Transaction::V2 { proofs, .. } => {
            fee = proofs
                .as_ref()
                .ok_or(TransactionError::TransactionVersionInvalid)?
                .base
                .fee;
        }
    }

    Ok(fee)
}
